use std::{os::unix::fs::MetadataExt, path::PathBuf};

use crate::app_source::AppSource;
use anyhow::{bail, Context, Ok, Result};
use clap::Parser;
use human_bytes::human_bytes;
use spin_locked_app::{
    locked::{LockedComponent, LockedMap, LockedTrigger},
    values::ValuesMap,
    Variable,
};
use spin_oci::OciLoader;
use tempfile::TempDir;
use walkdir::WalkDir;
use comfy_table::Table;
use serde_json::Value;


/// Get information about a Spin applicaton's metadata.
#[derive(Parser, Clone, Debug)]
pub struct InfoCommand {
    /// The application to display the information about.
    #[clap(name = "APPLICATION", short = 'f', long = "from", group = "source")]
    pub app_source: Option<String>,

    /// Cache directory for downloaded components and assets.
    #[clap(long)]
    pub cache_dir: Option<PathBuf>,
}

impl InfoCommand {
    pub async fn run(self) -> Result<()> {
        let app = self.app_source();
        match app {
            AppSource::OciRegistry(app) => self.print_info_registry(app).await,
            AppSource::File(app) => self.print_info_local(app).await,
            _ => bail!("Spin Info plugin only supports file or registry applications."),
        }
    }

    pub async fn print_info_registry(&self, app: String) -> Result<()> {
        println!("Getting info for app {:?}", app);

        let mut client = spin_oci::Client::new(false, self.cache_dir.clone())
            .await
            .context("cannot create registry client")?;

        let working_dir = TempDir::with_prefix("spin-info-")?;
        // TODO: because using `into_path()` here, the tmeporary directory is no longer cleaned up.
        let locked_app = OciLoader::new(working_dir.into_path())
            .load_app(&mut client, &app)
            .await?;

        self.print_metadata(&locked_app.metadata)?;

        println!("Application will be triggered by:");
        for t in &locked_app.triggers {
            self.print_trigger(t);
        }
        self.print_variables(&locked_app.variables);
        self.print_host_requirements(&locked_app.host_requirements);
        for c in &locked_app.components {
            self.print_component(c)?;
        }

        Ok(())
    }

    fn print_metadata(&self, meta: &ValuesMap) -> Result<()> {
        // TODO: because we're getting values from the values map,
        // the strings are quoted. Deserializing them to strings will
        // get rid of the extra quotes.
        let mut table = Table::new();
        table.set_header(vec!["Key", "Value"]);

        for(key, value) in meta.iter() {
            table.add_row(vec![&key, &value.to_string()]);
        }

        println!("Appliction Info");
        println!("{}", table);

        Ok(())
    }

    fn print_trigger(&self, trigger: &LockedTrigger) {
        // TODO: printing the trigger configuration should be prettier.
        println!(
            "   * {} trigger: {}: {}",
            trigger.trigger_type, trigger.id, trigger.trigger_config
        );
    }

    fn print_variables(&self, variables: &LockedMap<Variable>) {
        if !variables.is_empty() {
            println!("Variables:");
            for (k, v) in variables {
                println!("   * {}: {:?}", k, v);
            }
        }
    }

    fn print_host_requirements(&self, requirements: &ValuesMap) {
        if !requirements.is_empty() {
            println!("Host Requirements: {:?}", requirements);
        }
    }

    fn print_component(&self, component: &LockedComponent) -> Result<()> {
        println!("Component {}", component.id);

        let mut table = Table::new();
        table.set_header(vec!["Field", "Value"]);

        fn value_to_string(value: Option<&Value>) -> String {
            match value {
                Some(v) => match v {
                    Value::String(s) => s.clone(),
                    _ => v.to_string(),
                },
                None => "None".to_string(),
            }
        }

        table.add_row(vec![
            "Description", 
            &value_to_string(component.metadata.get("description"))
        ]);
        table.add_row(vec![
            "Allowed Outbound Hosts",
            &value_to_string(component.metadata.get("allowed_outbound_hosts")),
        ]);
        table.add_row(vec![
            "Allowed Key/Value Stores",
            &value_to_string(component.metadata.get("key_value_stores"))
                .replace("None", "[]"),
        ]);
        table.add_row(vec![
            "Allowed Databases",
            &value_to_string(component.metadata.get("databases"))
                .replace("None", "[]"),
        ]);
        table.add_row(vec![
            "Allowed AI Models",
            &value_to_string(component.metadata.get("ai_models")),
        ]);
    
        if let Some(build) = component.metadata.get("build") {
            table.add_row(vec!["Build Command", build.get("command").map_or("None", |v| v.as_str().unwrap_or_default())]);
        } else {
            table.add_row(vec!["Build Command", "None"]);
        }
    
        // Print component metadata table
        println!("Component Information:");
        println!("{}", table);

        let source = &component.source;
        println!("   The source for component {}", component.id);
        println!("      * content type: {}", source.content_type);
        let size = std::fs::metadata(
            source
                .content
                .source
                .clone()
                .expect("expected component to have wasm source")
                .strip_prefix("file://")
                .expect("expected source to be file URI"),
        )?
        .size() as f64;
        println!("      * file size: {}", human_bytes(size));

        if !&component.env.is_empty() {
            println!("   Environment variables:");
            for (k, v) in &component.env {
                println!("      * {}={}", k, v);
            }
        }

        if !&component.files.is_empty() {
            println!("   Files:");
            for f in &component.files {
                let mut count = 0;
                let mut size = 0;
                let path = &f.content.source.clone().expect("expected content source");
                for e in WalkDir::new(
                    path.strip_prefix("file://")
                        .expect("expected file source to be a file URI"),
                ) {
                    let e = e?;
                    if e.file_type().is_file() {
                        count += 1;
                        size += e.metadata()?.size();
                    }
                }
                println!(
                    "      * {} files mounted at path {:?}, {} in total",
                    count,
                    f.path,
                    human_bytes(size as f64)
                );
            }
        }

        Ok(())
    }

    pub async fn print_info_local(&self, _app: PathBuf) -> Result<()> {
        todo!("Printing information about a local application not implemented yet");
    }

    fn app_source(&self) -> AppSource {
        match &self.app_source {
            Some(src) => AppSource::infer_source(src),
            _ => AppSource::unresolvable("More than one application source was specified"),
        }
    }
}
