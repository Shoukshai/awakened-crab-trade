//! Backend-independent overlay policy. Network replies can only update existing content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Hidden,
    Preparing,
    Passive,
    Interactive,
    Closing,
    Error,
}

#[derive(Debug)]
pub struct Overlay {
    pub state: State,
    pub generation: u64,
    pub game_active: bool,
    pub rendered: bool,
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            state: State::Hidden,
            generation: 0,
            game_active: false,
            rendered: false,
        }
    }
}

impl Overlay {
    pub fn show(&mut self) -> Option<u64> {
        if !self.game_active {
            return None;
        }
        self.generation += 1;
        self.rendered = false;
        self.state = State::Preparing;
        Some(self.generation)
    }
    pub fn painted(&mut self) {
        if self.state == State::Preparing {
            self.rendered = true;
            self.state = State::Passive;
        }
    }
    pub fn interact(&mut self) -> bool {
        if self.state == State::Passive && self.rendered && self.game_active {
            self.state = State::Interactive;
            true
        } else {
            false
        }
    }
    pub fn hide(&mut self) {
        self.state = State::Closing;
        self.generation += 1;
        self.rendered = false;
        self.state = State::Hidden;
    }
    pub fn fail(&mut self) {
        self.hide();
        self.state = State::Error;
    }
    pub fn accepts(&self, generation: u64) -> bool {
        generation == self.generation
            && matches!(
                self.state,
                State::Preparing | State::Passive | State::Interactive
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn focus_requires_first_render_and_explicit_action() {
        let mut o = Overlay {
            game_active: true,
            ..Overlay::default()
        };
        o.show();
        assert!(!o.interact());
        o.painted();
        assert_eq!(o.state, State::Passive);
        assert!(o.interact());
    }
    #[test]
    fn late_network_reply_cannot_reopen_or_activate() {
        let mut o = Overlay {
            game_active: true,
            ..Overlay::default()
        };
        let g = o.show().unwrap();
        o.painted();
        o.hide();
        assert!(!o.accepts(g));
        assert!(!o.interact());
    }
    #[test]
    fn cannot_show_over_another_application() {
        assert_eq!(Overlay::default().show(), None);
    }
    #[test]
    fn failure_revokes_input_and_stale_replies() {
        let mut o = Overlay {
            game_active: true,
            ..Overlay::default()
        };
        let g = o.show().unwrap();
        o.painted();
        o.interact();
        o.fail();
        assert!(!o.rendered);
        assert!(!o.accepts(g));
        assert!(!o.interact());
    }
}
