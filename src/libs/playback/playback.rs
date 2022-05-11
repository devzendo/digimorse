use std::error::Error;
use log::{debug, info, warn};
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use bus::Bus;
use dashmap::DashMap;
use syncbox::{ScheduledThreadPool, Task};
use crate::libs::application::application::BusOutput;
use crate::libs::audio::tone_generator::{KeyingEventToneChannel, ToneGenerator};
use crate::libs::keyer_io::keyer_io::{KeyerEdgeDurationMs, KeyingEvent, KeyingTimedEvent};
use crate::libs::source_codec::keying_timing::{DefaultKeyingTiming, KeyingTiming};
use crate::libs::source_codec::source_encoding::{CallsignHash, Frame};
use crate::libs::util::util::get_epoch_ms;

const CHANNEL_LIFETIME_MS: u128 = 20000; // 20s enough?

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct StationIdentifier {
    callsign_hash: CallsignHash,
    audio_offset: u16,
}

#[derive(Debug)]
pub struct StationDetails {
    frames: Vec<Frame>,
    timing: Option<Box<dyn KeyingTiming>>,
    next_playback_schedule_time: u32,
    last_playback_end_epoch_ms: u128,
    current_polarity: bool,
    tone_generator_channel: usize,
    last_play_call_epoch_ms_for_channel_expiry: u128,
    send_start: bool,
}

pub struct Playback {
    terminate_flag: Arc<AtomicBool>,
    playback_state: DashMap<StationIdentifier, StationDetails>,
    tone_generator: Arc<Mutex<ToneGenerator>>,
    scheduled_thread_pool: Arc<ScheduledThreadPool>,
    output_tx: Arc<Mutex<Option<Arc<Mutex<Bus<KeyingEventToneChannel>>>>>>,
}

const BODGE_HACK_FIRST_FRAME_PLAYBACK_DELAY_MS: u32 = 1000;

// TODO possible future refactoring - Playback needs the ToneGenerator reference so it can
// allocate/deallocate channels for new/expiring received frames. It has a reference to the
// ToneGenerator's input channel so it can schedule sends to this channel. Perhaps these
// scheduled play methods could be exposed on the ToneGenerator, so that this channel isn't needed?
impl BusOutput<KeyingEventToneChannel> for Playback {
    fn clear_output_tx(&mut self) {
        match self.output_tx.lock() {
            Ok(mut locked) => {
                *locked = None;
            }
            Err(_) => {}
        }
    }

    fn set_output_tx(&mut self, output_tx: Arc<Mutex<Bus<KeyingEventToneChannel>>>) {
        match self.output_tx.lock() {
            Ok(mut locked) => { *locked = Some(output_tx); }
            Err(_) => {}
        }
    }
}

impl Playback {
    pub fn new(terminate: Arc<AtomicBool>, arc_scheduled_thread_pool: Arc<ScheduledThreadPool>, arc_tone_generator: Arc<Mutex<ToneGenerator>>) -> Self {
        let output_tx_holder: Arc<Mutex<Option<Arc<Mutex<Bus<KeyingEventToneChannel>>>>>> = Arc::new(Mutex::new(None));

        Self {
            terminate_flag: terminate,
            playback_state: DashMap::new(),
            tone_generator: arc_tone_generator,
            scheduled_thread_pool: arc_scheduled_thread_pool,
            output_tx: output_tx_holder,
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

    // The decoder will have taken a pass through the (possibly error-corrected) decode to find the
    // callsign hash (or computed it from a callsign), and already knows the audio offset.
    pub fn play(&mut self, decode: Result<Vec<Frame>, Box<dyn Error>>, callsign_hash: CallsignHash, audio_offset: u16) {
        let start_time = get_epoch_ms();
        let decode_ok_type = if decode.is_ok() { "frames" } else { "decode error" };
        debug!("Playing {} for callsign hash {} offset {} Hz", decode_ok_type, callsign_hash, audio_offset);
        let key = StationIdentifier { callsign_hash, audio_offset };
        if !self.playback_state.contains_key(&key) {
            debug!("New state for {:?}", key);
            let new_details = StationDetails {
                frames: vec![],
                timing: None,
                next_playback_schedule_time: 0,
                last_playback_end_epoch_ms: start_time,
                current_polarity: true,
                tone_generator_channel: self.tone_generator.lock().unwrap().allocate_channel(audio_offset),
                last_play_call_epoch_ms_for_channel_expiry: 0, // will be updated below...
                send_start: true,
            };
            self.playback_state.insert(key.clone(), new_details);
        } else {
            debug!("Existing state for {:?}", key);
        }
        match self.playback_state.get_mut(&key) {
            None => { panic!("StationDetails are present; shouldn't get here") }
            Some(mut details) => {
                // Store 'now' for the expiry handler.
                details.last_play_call_epoch_ms_for_channel_expiry = start_time;

                match decode {
                    Ok(frames) => {
                        for frame in frames {
                            info!("Playing back frame {:?}", frame);
                            match frame {
                                Frame::Padding => {}
                                Frame::WPMPolarity { wpm, polarity } => {
                                    let mut timing = Box::new(DefaultKeyingTiming::new());
                                    timing.set_keyer_speed(wpm);
                                    details.timing = Some(timing);
                                    details.current_polarity = polarity;
                                    if details.send_start { // TODO this may not be needed - try two transmissions
                                        let whom = details.value_mut();
                                        self.schedule_start(whom);
                                        details.send_start = false;
                                    }
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
                                Frame::KeyingPerfectWordgap => {
                                    match &details.timing {
                                        None => { warn!("No KeyingTiming set before {:?}", frame) }
                                        Some(timing) => {
                                            let duration_ms = timing.get_perfect_wordgap_ms();
                                            let whom = details.value_mut();
                                            self.schedule_tone(whom, duration_ms);
                                        }
                                    }
                                }
                                Frame::KeyingEnd => {
                                    let whom = details.value_mut();
                                    self.schedule_end(whom);
                                    details.send_start = true;
                                }
                                Frame::KeyingDeltaDit { delta } => {
                                    match &details.timing {
                                        None => { warn!("No KeyingTiming set before {:?}", frame) }
                                        Some(timing) => {
                                            let duration_ms = timing.get_perfect_dit_ms() as i16 + delta;
                                            let whom = details.value_mut();
                                            self.schedule_tone(whom, duration_ms as u16);
                                        }
                                    }
                                }
                                Frame::KeyingDeltaDah { delta } => {
                                    match &details.timing {
                                        None => { warn!("No KeyingTiming set before {:?}", frame) }
                                        Some(timing) => {
                                            let duration_ms = timing.get_perfect_dah_ms() as i16 + delta;
                                            let whom = details.value_mut();
                                            self.schedule_tone(whom, duration_ms as u16);
                                        }
                                    }
                                }
                                Frame::KeyingDeltaWordgap { delta } => {
                                    match &details.timing {
                                        None => { warn!("No KeyingTiming set before {:?}", frame) }
                                        Some(timing) => {
                                            let duration_ms = timing.get_perfect_wordgap_ms() as i16 + delta;
                                            let whom = details.value_mut();
                                            self.schedule_tone(whom, duration_ms as u16);
                                        }
                                    }
                                }
                                Frame::KeyingNaive { duration } => {
                                    let duration_ms = duration ;
                                    let whom = details.value_mut();
                                    self.schedule_tone(whom, duration_ms);
                                }
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

        self.expire();
    }

    pub fn expire(&mut self) {
        let oldest_activity_retained = get_epoch_ms() - CHANNEL_LIFETIME_MS;

        self.playback_state.retain(|key, value| {
            if value.last_play_call_epoch_ms_for_channel_expiry <= oldest_activity_retained {
                debug!("Expiring {:?}", key);
                self.tone_generator.lock().unwrap().deallocate_channel(value.tone_generator_channel);
                return false;
            }
            return true;
        });
    }

    fn schedule_start(&self, details: &mut StationDetails) {
        // This denotes the START of a tone.
        let now = get_epoch_ms();

        let last_playback_finished = now >= details.last_playback_end_epoch_ms;
        details.next_playback_schedule_time = if last_playback_finished {
            // If last playback has finished, this tone can start now at delta 0.
            BODGE_HACK_FIRST_FRAME_PLAYBACK_DELAY_MS // TODO improve this BODGE / HACK
            // However, if playback has finished and we're still in a transmission of tones, the
            // duration between details.last_playback_end_epoch_ms and now
            // needs to be factored into a playback delay feedback loop, otherwise we'll have gaps
            // in playback.
            // TODO introduce delay from playback feedback here.
        } else {
            // If last playback has yet to finish, start this tone immediately after it.
            // now < details.last_playback_end_epoch_ms
            (details.last_playback_end_epoch_ms - now) as u32
        };

        match self.output_tx.lock().unwrap().as_ref() {
            None => {}
            Some(output_tx) => {
                let cloned_output_tx = output_tx.clone();
                let ke = KeyingEvent::Start();
                let task = TimedPlayback { item: KeyingEventToneChannel { keying_event: ke, tone_channel: details.tone_generator_channel }, output_tx: cloned_output_tx };
                info!("!!! Scheduling start tone [ch# {}] @ time {}", details.tone_generator_channel, details.next_playback_schedule_time);
                self.scheduled_thread_pool.schedule_ms(details.next_playback_schedule_time, task);
            }
        }
        details.last_playback_end_epoch_ms = now + details.next_playback_schedule_time as u128; // really only matters for first frame, subsequent will not change this
    }

    fn schedule_tone(&self, details: &mut StationDetails, duration_ms: KeyerEdgeDurationMs) {
        // Compute feedback delay
        let now = get_epoch_ms();
        let last_playback_finished = now >= details.last_playback_end_epoch_ms;
        details.next_playback_schedule_time = (if last_playback_finished {
            // now >= details.last_playback_end_epoch_ms
            // It's OK for this to be the
            let gap_duration = now - details.last_playback_end_epoch_ms;
            // TODO set delay from playback feedback here.
            warn!("Tone scheduled {} ms after last tone playback", gap_duration);
            0
        } else {
            // If last playback has yet to finish, start this tone immediately after it.
            // now < details.last_playback_end_epoch_ms
            details.last_playback_end_epoch_ms - now
        } + duration_ms as u128) as u32;
        // This denotes the END of a tone.

        match self.output_tx.lock().unwrap().as_ref() {
            None => {}
            Some(output_tx) => {
                let cloned_output_tx = output_tx.clone();
                let ke = KeyingEvent::Timed(KeyingTimedEvent { up: details.current_polarity, duration: duration_ms });
                let task = TimedPlayback { item: KeyingEventToneChannel { keying_event: ke, tone_channel: details.tone_generator_channel }, output_tx: cloned_output_tx };
                info!("!!! Scheduling end of tone [ch# {}] {} after {}ms @ time {:?}", details.tone_generator_channel, ( if details.current_polarity { "MARK ^" } else { "SPACE v" } ), duration_ms, details.next_playback_schedule_time);
                self.scheduled_thread_pool.schedule_ms(details.next_playback_schedule_time, task);
            }
        }
        details.current_polarity = !details.current_polarity;
        details.last_playback_end_epoch_ms += duration_ms as u128;
    }

    fn schedule_end(&self, details: &mut StationDetails) {
        let now = get_epoch_ms();
        let last_playback_finished = now >= details.last_playback_end_epoch_ms;
        details.next_playback_schedule_time = (if last_playback_finished {
            // now >= details.last_playback_end_epoch_ms
            // It's OK for this to be the
            let gap_duration = now - details.last_playback_end_epoch_ms;
            // TODO set delay from playback feedback here.
            warn!("Tone scheduled {} ms after last tone playback", gap_duration);
            0
        } else {
            // If last playback has yet to finish, start this tone immediately after it.
            // now < details.last_playback_end_epoch_ms
            details.last_playback_end_epoch_ms - now
        }) as u32;
        // This denotes the END of a tone.

        details.current_polarity = true;
        match self.output_tx.lock().unwrap().as_ref() {
            None => {}
            Some(output_tx) => {
                let cloned_output_tx = output_tx.clone();
                let ke = KeyingEvent::End();
                let task = TimedPlayback { item: KeyingEventToneChannel { keying_event: ke, tone_channel: details.tone_generator_channel }, output_tx: cloned_output_tx };
                info!("!!! Scheduling end on tone [ch# {}] @ time {:?}", details.tone_generator_channel, details.next_playback_schedule_time);
                self.scheduled_thread_pool.schedule_ms(details.next_playback_schedule_time, task);
            }
        }
    }

    #[cfg(test)]
    fn get_last_playback_schedule_time(&self, callsign_hash: CallsignHash, audio_offset: u16) -> Option<u32> {
        let key = StationIdentifier { callsign_hash, audio_offset };
        match self.playback_state.get(&key) {
            None => { None }
            Some(thing) => { Some(thing.value().next_playback_schedule_time) }
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

#[cfg(test)]
#[path = "./playback_from_keying_spec.rs"]
mod playback_from_keying_spec;
