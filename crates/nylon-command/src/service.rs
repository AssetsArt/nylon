use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum ServiceCommands {
    // Install the service
    #[command(name = "install")]
    #[command(about = "Install the service.")]
    Install,

    // Uninstall the service
    #[command(name = "uninstall")]
    #[command(about = "Uninstall the service.")]
    Uninstall,

    // Start the service
    #[command(name = "start")]
    #[command(about = "Start the service and apply all configuration changes.")]
    Start,

    // Stop the service
    #[command(name = "stop")]
    #[command(about = "Stop the service.")]
    Stop,

    // Restart the service
    #[command(name = "restart")]
    #[command(about = "Restart the service and apply all configuration changes.")]
    Restart,

    // Status of the service
    #[command(name = "status")]
    #[command(about = "Show the status of the service.")]
    Status,

    // Reload the service
    #[command(name = "reload")]
    #[command(
        about = "Reload route and service configurations without applying global configuration changes."
    )]
    Reload,
}
