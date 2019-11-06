use super::addons::*;
use crate::state_types::msg::Internal::*;
use crate::state_types::*;
use crate::types::addons::{AggrRequest, ResourceRef, ResourceRequest};
use crate::types::{MetaDetail, Stream};
use serde_derive::*;

#[derive(Debug, Clone, Default, Serialize)]
pub struct MetaDetails {
    pub selected: Option<(ResourceRef, Option<ResourceRef>)>,
    pub metas: Vec<ItemsGroup<MetaDetail>>,
    pub streams: Vec<ItemsGroup<Vec<Stream>>>,
}

impl<Env: Environment + 'static> UpdateWithCtx<Ctx<Env>> for MetaDetails {
    fn update(&mut self, ctx: &Ctx<Env>, msg: &Msg) -> Effects {
        match msg {
            Msg::Action(Action::Load(ActionLoad::MetaDetails {
                type_name,
                id,
                video_id,
            })) => {
                let metas_resource_ref = ResourceRef::without_extra("meta", type_name, id);
                let metas_effects = if self.metas.first().map_or(true, |metas_group| {
                    metas_group.req.path != metas_resource_ref
                }) {
                    let (metas, metas_effects) = addon_aggr_new::<Env, _>(
                        &ctx.content.addons,
                        &AggrRequest::AllOfResource(metas_resource_ref.clone()),
                    );
                    self.metas = metas;
                    metas_effects
                } else {
                    Effects::none().unchanged()
                };
                if let Some(video_id) = video_id {
                    let streams_resource_ref =
                        ResourceRef::without_extra("stream", type_name, video_id);
                    let streams_effects = if self.streams.first().map_or(true, |streams_group| {
                        streams_group.req.path != streams_resource_ref
                    }) {
                        let (streams, streams_effects) = addon_aggr_new::<Env, _>(
                            &ctx.content.addons,
                            &AggrRequest::AllOfResource(streams_resource_ref.clone()),
                        );
                        self.streams = streams;
                        streams_effects
                    } else {
                        Effects::none().unchanged()
                    };
                    self.selected = Some((metas_resource_ref, Some(streams_resource_ref)));
                    metas_effects.join(streams_effects)
                } else {
                    self.streams = Vec::new();
                    self.selected = Some((metas_resource_ref, None));
                    metas_effects
                }
            }
            Msg::Internal(AddonResponse(_, _)) => {
                let metas_effects = addon_aggr_update(&mut self.metas, msg);
                let streams_effects: Effects =
                    if let Some((_, Some(streams_resource_ref))) = &self.selected {
                        if let Some((meta_transport_url, streams_from_meta)) = self
                            .metas
                            .iter()
                            .find_map(|metas_group| match &metas_group.content {
                                Loadable::Ready(meta_item) => {
                                    meta_item.videos.iter().find_map(|video| {
                                        if video.id == streams_resource_ref.id
                                            && !video.streams.is_empty()
                                        {
                                            Some((
                                                metas_group.req.base.to_owned(),
                                                video.streams.to_owned(),
                                            ))
                                        } else {
                                            None
                                        }
                                    })
                                }
                                _ => None,
                            })
                        {
                            self.streams = vec![ItemsGroup {
                                req: ResourceRequest {
                                    base: meta_transport_url,
                                    path: streams_resource_ref.to_owned(),
                                },
                                content: Loadable::Ready(streams_from_meta),
                            }];
                            Effects::none()
                        } else {
                            addon_aggr_update(&mut self.streams, msg)
                        }
                    } else {
                        Effects::none().unchanged()
                    };

                metas_effects.join(streams_effects)
            }
            _ => Effects::none().unchanged(),
        }
    }
}