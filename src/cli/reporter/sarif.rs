use super::Reporter;
use crate::core::{
    language::Queryable,
    matcher::MatchedItem,
    ruleset::{Rule, Severity},
    target::Target,
};
use anyhow::Result;
use serde_sarif::sarif;
use std::{collections::HashMap, convert::TryInto};

pub struct SARIFReporter<'a, Writer: std::io::Write> {
    writer: &'a mut Writer,

    results: Vec<sarif::Result>,

    descriptors_idx_map: HashMap<String, usize>,
    descriptors: Vec<sarif::ReportingDescriptor>,
}

impl<'a, W: std::io::Write> Reporter<'a> for SARIFReporter<'a, W> {
    type Writer = W;
    fn new(writer: &'a mut Self::Writer) -> Self {
        Self {
            writer,

            results: vec![],

            descriptors_idx_map: HashMap::new(),
            descriptors: vec![],
        }
    }

    fn add_entry<T: Queryable + 'static>(
        &mut self,
        target: &Target,
        items: Vec<(&Rule, MatchedItem)>,
    ) -> Result<()> {
        for (rule, mitem) in items {
            let descriptor_idx = {
                if let Some(idx) = self.descriptors_idx_map.get(&rule.id) {
                    *idx
                } else {
                    let descriptor = sarif::ReportingDescriptorBuilder::default()
                        .id(rule.id.clone())
                        .short_description::<sarif::MultiformatMessageString>(
                            sarif::MultiformatMessageStringBuilder::default()
                                .markdown(rule.message.clone())
                                .text(rule.message.clone())
                                .build()?,
                        )
                        .full_description::<sarif::MultiformatMessageString>(
                            sarif::MultiformatMessageStringBuilder::default()
                                .markdown(rule.message.clone())
                                .text(rule.message.clone())
                                .build()?,
                        )
                        .help(
                            sarif::MultiformatMessageStringBuilder::default()
                                .markdown(rule.message.clone())
                                .text(rule.message.clone())
                                .build()?,
                        )
                        .build()?;
                    self.descriptors.push(descriptor);
                    let idx = self.descriptors.len() - 1;
                    self.descriptors_idx_map.insert(rule.id.clone(), idx);
                    idx
                }
            };

            let result = sarif::ResultBuilder::default()
                .rule_id(rule.id.clone())
                .rule_index(descriptor_idx as i64)
                .message::<sarif::Message>(
                    sarif::MessageBuilder::default()
                        .markdown(rule.message.clone())
                        .text(rule.message.clone())
                        .build()?,
                )
                .locations(vec![sarif::LocationBuilder::default()
                    .physical_location(
                        sarif::PhysicalLocationBuilder::default()
                            .artifact_location(
                                sarif::ArtifactLocationBuilder::default()
                                    .uri(target.relative_path())
                                    .build()?,
                            )
                            .region(
                                sarif::RegionBuilder::default()
                                    .start_line(mitem.area.range::<T>().start.row as i64)
                                    .start_column(mitem.area.range::<T>().start.column as i64)
                                    .build()?,
                            )
                            .build()?,
                    )
                    .build()?])
                .level(
                    match rule.get_level() {
                        Severity::Unknown => sarif::ResultLevel::None,
                        Severity::Low => sarif::ResultLevel::Note,
                        Severity::Medium => sarif::ResultLevel::Warning,
                        Severity::High => sarif::ResultLevel::Error,
                        Severity::Critical => sarif::ResultLevel::Error,
                    }
                    .to_string(),
                )
                .build()?;
            self.results.push(result);
        }

        Ok(())
    }

    fn report(&mut self) -> Result<()> {
        let tool_component: sarif::ToolComponent = sarif::ToolComponentBuilder::default()
            .name("shisho")
            .version(env!("CARGO_PKG_VERSION"))
            .information_uri("https://docs.shisho.dev")
            .rules(self.descriptors.clone())
            .build()?;

        let run = sarif::RunBuilder::default()
            .tool::<sarif::Tool>(tool_component.try_into()?)
            .results(self.results.clone())
            .build()?;

        let sarif = sarif::SarifBuilder::default()
            .version(sarif::Version::V2_1_0.to_string())
            .schema("https://docs.oasis-open.org/sarif/sarif/v2.1.0/cos02/schemas/sarif-schema-2.1.0.json")
            .runs(vec![run])
            .build()?;

        let s = serde_json::to_string(&sarif)?;
        write!(self.writer, "{}", s)?;

        self.results = vec![];
        self.descriptors = vec![];
        self.descriptors_idx_map = HashMap::new();
        Ok(())
    }
}
