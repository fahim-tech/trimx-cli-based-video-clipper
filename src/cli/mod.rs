// CLI adapter - Primary adapter for command-line interface

use crate::domain::model::*;
use crate::domain::errors::*;
use crate::app::*;
use crate::adapters::*;
use clap::Parser;

/// CLI adapter - Primary adapter for command-line interface
pub struct CliAdapter {
    clip_interactor: ClipInteractor,
    inspect_interactor: InspectInteractor,
    verify_interactor: VerifyInteractor,
}

impl CliAdapter {
    /// Create new CLI adapter with injected interactors
    pub fn new(
        clip_interactor: ClipInteractor,
        inspect_interactor: InspectInteractor,
        verify_interactor: VerifyInteractor,
    ) -> Self {
        Self {
            clip_interactor,
            inspect_interactor,
            verify_interactor,
        }
    }
    
    /// Run CLI application
    pub async fn run(&self) -> Result<(), DomainError> {
        let cli = Cli::parse();
        
        match cli.command {
            Commands::Clip(args) => {
                self.handle_clip_command(args).await?;
            }
            Commands::Inspect(args) => {
                self.handle_inspect_command(args).await?;
            }
            Commands::Verify(args) => {
                self.handle_verify_command(args).await?;
            }
        }
        
        Ok(())
    }
    
    /// Handle clip command
    async fn handle_clip_command(&self, args: ClipArgs) -> Result<(), DomainError> {
        let cut_range = CutRange::new(
            self.parse_time(&args.start)?,
            self.parse_time(&args.end)?,
        )?;
        
        let request = ClipRequest::new(
            args.input,
            args.output.unwrap_or_default(),
            cut_range,
            self.parse_mode(&args.mode)?,
        )?;
        
        let response = self.clip_interactor.execute(request).await?;
        
        if response.success {
            println!("✅ Successfully clipped video!");
            println!("   Output: {}", response.output_file);
            println!("   Duration: {}", response.duration);
            println!("   Mode used: {:?}", response.mode_used);
            println!("   Processing time: {:.2}s", response.processing_time.as_secs_f64());
            
            if !response.warnings.is_empty() {
                println!("   Warnings:");
                for warning in &response.warnings {
                    println!("     - {}", warning);
                }
            }
        } else {
            return Err(DomainError::ExecFail("Clipping failed".to_string()));
        }
        
        Ok(())
    }
    
    /// Handle inspect command
    async fn handle_inspect_command(&self, args: InspectArgs) -> Result<(), DomainError> {
        let request = InspectRequest::new(args.input);
        
        let response = self.inspect_interactor.execute(request).await?;
        
        if args.json {
            let json_report = self.inspect_interactor.generate_json_report(&response)?;
            println!("{}", json_report);
        } else {
            let report = self.inspect_interactor.generate_report(&response);
            println!("{}", report);
        }
        
        Ok(())
    }
    
    /// Handle verify command
    async fn handle_verify_command(&self, args: VerifyArgs) -> Result<(), DomainError> {
        let expected_range = CutRange::new(
            self.parse_time(&args.start)?,
            self.parse_time(&args.end)?,
        )?;
        
        let request = VerifyRequest::new(
            args.input,
            expected_range,
            self.parse_mode(&args.mode)?,
        );
        
        let response = self.verify_interactor.execute(request).await?;
        
        if response.verification_result.success {
            println!("✅ Verification successful!");
            println!("   Overall score: {:.1}%", response.verification_result.overall_score);
        } else {
            println!("❌ Verification failed!");
            println!("   Error: {}", response.verification_result.error_message);
            println!("   Overall score: {:.1}%", response.verification_result.overall_score);
        }
        
        println!("\nDetailed checks:");
        for check in &response.verification_result.checks {
            let status = if check.success { "✅" } else { "❌" };
            println!("   {} {}: {}", status, check.check_type, check.details);
            if !check.error_message.is_empty() {
                println!("      Error: {}", check.error_message);
            }
        }
        
        Ok(())
    }
    
    /// Parse time string to TimeSpec
    fn parse_time(&self, time_str: &str) -> Result<TimeSpec, DomainError> {
        TimeSpec::parse(time_str)
    }
    
    /// Parse mode string to ClippingMode
    fn parse_mode(&self, mode_str: &str) -> Result<ClippingMode, DomainError> {
        ClippingMode::parse(mode_str)
    }
}

/// CLI command structure
#[derive(Parser)]
#[command(name = "trimx")]
#[command(about = "A Windows-native command-line tool for precise video clipping")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands
#[derive(Parser)]
pub enum Commands {
    /// Extract video segments
    Clip(ClipArgs),
    /// Inspect video file information
    Inspect(InspectArgs),
    /// Verify a clipped segment
    Verify(VerifyArgs),
}

/// Clip command arguments
#[derive(Parser)]
pub struct ClipArgs {
    /// Input video file path
    #[arg(short, long)]
    pub input: String,
    
    /// Start time (HH:MM:SS.ms or MM:SS.ms or seconds)
    #[arg(short, long)]
    pub start: String,
    
    /// End time (HH:MM:SS.ms or MM:SS.ms or seconds)
    #[arg(short, long)]
    pub end: String,
    
    /// Output file path
    #[arg(short, long)]
    pub output: Option<String>,
    
    /// Clipping mode (auto, copy, reencode, hybrid)
    #[arg(short, long, default_value = "auto")]
    pub mode: String,
}

/// Inspect command arguments
#[derive(Parser)]
pub struct InspectArgs {
    /// Input video file path
    #[arg(short, long)]
    pub input: String,
    
    /// Output in JSON format
    #[arg(long)]
    pub json: bool,
}

/// Verify command arguments
#[derive(Parser)]
pub struct VerifyArgs {
    /// Input video file path
    #[arg(short, long)]
    pub input: String,
    
    /// Start time (HH:MM:SS.ms or MM:SS.ms or seconds)
    #[arg(short, long)]
    pub start: String,
    
    /// End time (HH:MM:SS.ms or MM:SS.ms or seconds)
    #[arg(short, long)]
    pub end: String,
    
    /// Expected mode (auto, copy, reencode, hybrid)
    #[arg(short, long, default_value = "auto")]
    pub mode: String,
}