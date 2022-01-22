use std::error::Error;
use log::{debug, warn};
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use bus::Bus;
use dashmap::DashMap;
use syncbox::{ScheduledThreadPool, Task};
use crate::libs::audio::tone_generator::{KeyingEventToneChannel, ToneGenerator};
use crate::libs::keyer_io::keyer_io::{KeyerEdgeDurationMs, KeyingEvent, KeyingTimedEvent};
use crate::libs::source_codec::keying_timing::{DefaultKeyingTiming, KeyingTiming};
use crate::libs::source_codec::source_encoding::{CallsignHash, Frame};
use crate::libs::util::test_util::get_epoch_ms;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct StationIdentifier {
    callsign_hash: CallsignHash,
    audio_offset: u16,
}

#[derive(Debug)]
pub struct StationDetails {
    frames: Vec<Frame>,
    timing: Option<Box<dyn KeyingTiming>>,
    last_playback_schedule_time: u32,
    current_polarity: bool,
}

pub struct Playback {
    terminate_flag: Arc<AtomicBool>,
    playback_state: DashMap<StationIdentifier, StationDetails>,
    tone_generator: Arc<Mutex<ToneGenerator>>,
    scheduled_thread_pool: Arc<ScheduledThreadPool>,
    keying_event_tone_channel_tx: Arc<Mutex<Bus<KeyingEventToneChannel>>>,
}

impl Playback {
    pub fn new(terminate: Arc<AtomicBool>, arc_scheduled_thread_pool: Arc<ScheduledThreadPool>, arc_tone_generator: Arc<Mutex<ToneGenerator>>, keying_event_tone_channel_tx: Arc<Mutex<Bus<KeyingEventToneChannel>>>) -> Self {
        Self {
            terminate_flag: terminate,
            playback_state: DashMap::new(),
            tone_generator: arc_tone_generator,
            scheduled_thread_pool: arc_scheduled_thread_pool,
            keying_event_tone_channel_tx,
        }
    }

    // Signals the thread to terminate, blocks on joining the handle. Used by drop().
    // Setting the terminate AtomicBool will allow the thread to stop on its own, but there's no
    // method other than this for blocking until it has actually stopped.
    pub fn terminate(&mut self) {
        debug!("Terminating playback");
        self.terminate_flag.store(true, core::sync::atomic::Ordering::SeqCst);
        debug!("Playback terminated");
    }

    pub fn play(&mut self, decode: Result<Vec<Frame>, Box<dyn Error>>, callsign_hash: CallsignHash, audio_offset: u16) {
        let decode_ok_type = if decode.is_ok() { "frames" } else { "decode error" };
        debug!("Playing {} for callsign hash {} offset {} Hz", decode_ok_type, callsign_hash, audio_offset);
        let key = StationIdentifier { callsign_hash, audio_offset };
        if !self.playback_state.contains_key(&key) {
            debug!("New state for {:?}", key);
            let new_details = StationDetails {
                frames: vec![],
                timing: None,
                last_playback_schedule_time: 0,
                current_polarity: true,
            };
            self.playback_state.insert(key.clone(), new_details);
        } else {
            debug!("Existing state for {:?}", key);
        }
        match self.playback_state.get_mut(&key) {
            None => { panic!("StationDetails are present; shouldn't get here") }
            Some(mut details) => {
                let start_time = get_epoch_ms();
                match decode {
                    Ok(frames) => {
                        for frame in frames {
                            debug!("Playing back frame {:?}", frame);
                            match frame {
                                Frame::Padding => {}
                                Frame::WPMPolarity { wpm, polarity } => {
                                    let mut timing = Box::new(DefaultKeyingTiming::new());
                                    timing.set_keyer_speed(wpm);
                                    details.timing = Some(timing);
                                    details.current_polarity = polarity;
                                }
                                Frame::CallsignMetadata { .. } => {}
                                Frame::CallsignHashMetadata { .. } => {}
                                Frame::LocatorMetadata { .. } => {}
                                Frame::KeyingPerfectDit => {
                                    match &details.timing {
                                        None => { warn!("No KeyingTiming set before {:?}", frame) }
                                        Some(timing) => {
                                            let duration_ms = timing.get_perfect_dit_ms();
                                            let whom = details.value_mut();
                                            self.schedule_tone(whom, duration_ms);
                                        }
                                    }
                                }
                                Frame::KeyingPerfectDah => {
                                    match &details.timing {
                                        None => { warn!("No KeyingTiming set before {:?}", frame) }
                                        Some(timing) => {
                                            let duration_ms = timing.get_perfect_dah_ms();
                                            let whom = details.value_mut();
                                            self.schedule_tone(whom, duration_ms);
                                        }
                                    }
                                }
                                Frame::KeyingPerfectWordgap => {}
                                Frame::KeyingEnd => {
                                    let whom = details.value_mut();
                                    self.schedule_end(whom);
                                }
                                Frame::KeyingDeltaDit { .. } => {}
                                Frame::KeyingDeltaDah { .. } => {}
                                Frame::KeyingDeltaWordgap { .. } => {}
                                Frame::KeyingNaive { .. } => {}
                                Frame::Unused => {}
                                Frame::Extension => {}
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Cannot playback a decode error {}", e);
                    }
                }
                let end_time = get_epoch_ms();
                debug!("Frame playback took {}ms", end_time - start_time);
            }
        }
    }

    fn schedule_tone(&self, details: &mut StationDetails, duration_ms: KeyerEdgeDurationMs) {
        let chan = self.keying_event_tone_channel_tx.clone();

        // TODO obvs, allocate a channel and store it in the details
        let bodged_sidetone_channel = 0;

        details.current_polarity = !details.current_polarity;
        let ke = KeyingEvent::Timed(KeyingTimedEvent { up: details.current_polarity, duration: duration_ms });
        let task = TimedPlayback { item: KeyingEventToneChannel { keying_event: ke, tone_channel: bodged_sidetone_channel }, output_tx: chan };
        debug!("Scheduling tone channel {} for {}ms at time {}", bodged_sidetone_channel, duration_ms, details.last_playback_schedule_time);
        self.scheduled_thread_pool.schedule_ms(details.last_playback_schedule_time, task);
        details.last_playback_schedule_time += duration_ms as u32;
    }

    fn schedule_end(&self, details: &mut StationDetails) {
        let chan = self.keying_event_tone_channel_tx.clone();

        // TODO obvs, allocate a channel and store it in the details
        let bodged_sidetone_channel = 0;

        details.current_polarity = true;
        let ke = KeyingEvent::Timed(KeyingTimedEvent { up: details.current_polarity, duration: 0 });
        let task = TimedPlayback { item: KeyingEventToneChannel { keying_event: ke, tone_channel: bodged_sidetone_channel }, output_tx: chan };
        debug!("Scheduling end on tone channel {} at time {}", bodged_sidetone_channel, details.last_playback_schedule_time);
        self.scheduled_thread_pool.schedule_ms(details.last_playback_schedule_time, task);
    }

    fn get_last_playback_schedule_time(&self, callsign_hash: CallsignHash, audio_offset: u16) -> Option<u32> {
        let key = StationIdentifier { callsign_hash, audio_offset };
        match self.playback_state.get(&key) {
            None => { None }
            Some(thing) => { Some(thing.value().last_playback_schedule_time) }
        }
    }
}

struct TimedPlayback {
    item: KeyingEventToneChannel,
    output_tx: Arc<Mutex<Bus<KeyingEventToneChannel>>>,
}

impl Task for TimedPlayback {
    fn run(self) {
        //debug!("TimedPlayback playing {}", self.item);
        let mut output = self.output_tx.lock().unwrap();
        output.broadcast(self.item);
    }
}

#[cfg(test)]
#[path = "./playback_spec.rs"]
mod playback_spec;
