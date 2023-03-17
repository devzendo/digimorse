#[derive(Clone, Copy, Debug)]
pub enum Message {
    Create,
    Update,
    Delete,
    Select,
    Filter,
}
