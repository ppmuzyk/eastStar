pub trait VisualSession {
    fn prepare(&self);
    fn show(&self);
    fn hide(&self);
}

pub struct PlaceholderVisual;

impl VisualSession for PlaceholderVisual {
    fn prepare(&self) {
        println!("visual: prepare placeholder scene");
    }

    fn show(&self) {
        println!("visual: show placeholder fullscreen scene");
    }

    fn hide(&self) {
        println!("visual: hide placeholder scene");
    }
}
