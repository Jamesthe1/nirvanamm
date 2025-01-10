use winsafe::gui::WindowControl;

// Because AsRef<Self> is not implemented in WindowControl, we *have* to wrap it.
#[derive(Clone)]
pub struct WindowControlWrapper(WindowControl);

impl AsRef<WindowControl> for WindowControlWrapper {
    fn as_ref(&self) -> &WindowControl {
        &self.0
    }
}

impl WindowControlWrapper {
    #[must_use]
    pub fn new(wc: WindowControl) -> Self {
        Self(wc)
    }
}