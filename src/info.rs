use std::path::PathBuf;

use crate::app_source::AppSource;
use anyhow::{bail, Context, Result};
use clap::Parser;
use spin_locked_app::{
    locked::{LockedApp, LockedComponent, LockedMap, LockedTrigger},
    values::ValuesMap,
    Variable,
};
use spin_oci::OciLoader;
use tempfile::TempDir;

#[derive(Parser, Clone, Debug)]
pub struct InfoCommand {
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
            AppSource::File(app) => self.print_info_local(app).await,
            AppSource::OciRegistry(app) => self.print_info_registry(app).await,
            _ => bail!("Spin Info plugin only supports file or registry applications."),
        }
    }

    pub async fn print_info_registry(&self, app: String) -> Result<()> {
        println!("Getting info for app {:?}", app);

        let mut client = spin_oci::Client::new(false, self.cache_dir.clone())
            .await
            .context("cannot create registry client")?;

        let working_dir = TempDir::with_prefix("spin-info-")?;
        let locked_app = OciLoader::new(&working_dir.path())
            .load_app(&mut client, &app)
            .await?;

        // println!("{:?}", locked_app);

        self.print_metadata(&locked_app);

        for t in &locked_app.triggers {
            self.print_trigger(t);
        }
        self.print_variables(&locked_app.variables);
        self.print_host_requirements(&locked_app.host_requirements);
        for c in &locked_app.components {
            self.print_component(c);
        }

        Ok(())
    }

    fn print_metadata(&self, app: &LockedApp) {
        println!("Application metadata: ");
        for (k, v) in &app.metadata {
            println!("      {}: {}", k, v);
        }
    }

    fn print_trigger(&self, trigger: &LockedTrigger) {
        println!("Trigger: {:?}", trigger);
    }

    fn print_variables(&self, variables: &LockedMap<Variable>) {
        println!("Variables: {:?}", variables);
    }

    fn print_host_requirements(&self, requirements: &ValuesMap) {
        println!("Host Requirements: {:?}", requirements);
    }

    fn print_component(&self, component: &LockedComponent) {
        println!("{:?}", component);
    }

    pub async fn print_info_local(&self, app: PathBuf) -> Result<()> {
        println!("Getting info for app {:?}", app);
        Ok(())
    }

    fn app_source(&self) -> AppSource {
        match &self.app_source {
            Some(src) => AppSource::infer_source(src),
            _ => AppSource::unresolvable("More than one application source was specified"),
        }
    }
}
