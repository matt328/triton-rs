use anyhow::Context;

use triton::Application;

fn main() -> anyhow::Result<()> {
    log4rs::init_file("log4rs.yml", Default::default()).context("Could not configure logger")?;

    let app = Application::new().context("Failed to create Application")?;

    app.run()
}
