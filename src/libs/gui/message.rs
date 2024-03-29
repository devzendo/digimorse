use crate::libs::keyer_io::keyer_io::KeyerSpeed;

#[derive(Clone, Debug)]
pub struct KeyingText {
    pub text: String,
}

#[derive(Clone, Debug)]
pub enum Message {
    KeyingText(KeyingText),
    Beep,
    IncreaseKeyingSpeedRequest,
    DecreaseKeyingSpeedRequest,
    SetKeyingSpeed(KeyerSpeed),
    RedrawIndicatorsCanvas,
    SetRxIndicator(bool),
    SetWaitIndicator(bool),
    SetTxIndicator(bool),
}
