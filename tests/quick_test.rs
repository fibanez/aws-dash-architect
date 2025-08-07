use awsdash::app::compliance_discovery::ComplianceDiscovery; 
use anyhow::Result;

#[tokio::test]
async fn quick_test_json_parsing() -> Result<()> {
    let mut discovery = ComplianceDiscovery::new_with_default_cache();
    let programs = discovery.discover_available_programs().await?;
    println\!("Found {} programs", programs.len());
    for program in programs.iter().take(3) {
        println\!("- {} ({})", program.display_name, program.name);
    }
    Ok(())
}
