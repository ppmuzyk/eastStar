pub trait SessionLocker {
    fn lock(&self);
}

pub struct NoopLocker;

impl SessionLocker for NoopLocker {
    fn lock(&self) {
        println!("lock: placeholder trigger, no desktop lock integration is wired yet");
    }
}
