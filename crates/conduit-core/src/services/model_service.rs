use crate::dto::{ListModelsDto, ModelGroupDto, ModelInfoDto};
use crate::services::config_service::ConfigService;
use crate::services::ContextWindowService;
use crate::ConduitCore;
use conduit_agent::ModelRegistry;

pub struct ModelService;

impl ModelService {
    pub fn list_models(core: &ConduitCore) -> ListModelsDto {
        let agent_types = core.config().effective_enabled_providers(core.tools());
        let (default_agent, default_model) = ConfigService::default_model(core);

        let groups = agent_types
            .iter()
            .filter_map(|&agent_type| {
                let models = ModelRegistry::models_for(agent_type);
                if models.is_empty() {
                    return None;
                }
                let agent_type_str = agent_type.as_str().to_string();

                let model_entries = models
                    .into_iter()
                    .map(|model| {
                        let is_default =
                            model.agent_type == default_agent && model.id == default_model;
                        let resolved =
                            ContextWindowService::resolve(core, model.agent_type, &model.id);
                        ModelInfoDto {
                            id: model.id,
                            display_name: model.display_name,
                            description: model.description,
                            is_default,
                            agent_type: agent_type_str.clone(),
                            context_window: resolved.tokens,
                        }
                    })
                    .collect();

                Some(ModelGroupDto {
                    agent_type: agent_type_str,
                    section_title: ModelRegistry::agent_section_title(agent_type).to_string(),
                    icon: ModelRegistry::agent_icon(agent_type).to_string(),
                    models: model_entries,
                })
            })
            .collect();

        ListModelsDto { groups }
    }
}
