use colored::Colorize;

pub fn log_section(title: &str) {
    println!("\n{}", "━".repeat(50).bright_black());
    println!("{}", title.bright_blue().bold());
    println!("{}", "━".repeat(50).bright_black());
}

pub fn log_success(message: &str) {
    println!("{} {}", "✓".green(), message);
}

pub fn log_info(message: &str) {
    println!("{} {}", "ℹ".blue(), message);
}

pub fn log_warning(message: &str) {
    println!("{} {}", "⚠".yellow(), message);
}
