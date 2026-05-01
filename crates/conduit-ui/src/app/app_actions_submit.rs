use crate::action::Action;
use crate::app::App;
use crate::effect::Effect;

impl App {
    pub(super) fn handle_submit_related_action(
        &mut self,
        action: Action,
        effects: &mut Vec<Effect>,
    ) -> anyhow::Result<()> {
        match action {
            Action::Submit => {
                effects
                    .extend(self.handle_submit_action(conduit_data::QueuedMessageMode::FollowUp)?);
            }
            Action::SubmitSteer => {
                effects.extend(self.handle_submit_action(conduit_data::QueuedMessageMode::Steer)?);
            }
            _ => {}
        }

        Ok(())
    }
}
