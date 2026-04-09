//! Built-in tools for the chat assistant

use rig::tool::ToolError;
use rig::tool_macro;

// Get the current time
#[tool_macro]
pub fn get_time() -> Result<String, ToolError> {
    let now = chrono::Local::now();
    Ok(now.format("%Y-%m-%d %H:%M:%S").to_string())
}

// Get the weather for a city
#[tool_macro(required(city))]
pub fn get_weather(city: String) -> Result<String, ToolError> {
    Ok(format!("{} today: Sunny, 25°C, humidity 60%", city))
}

// Calculator tool
#[tool_macro(required(expression))]
pub fn calculate(expression: String) -> Result<f64, ToolError> {
    expression
        .replace(" ", "")
        .parse()
        .map_err(|e| ToolError::ToolCallError(format!("Parse error: {}", e).into()))
}

// Set a reminder or alarm
#[tool_macro(required(message))]
pub fn set_reminder(message: String, minutes: Option<i32>) -> Result<String, ToolError> {
    let mins = minutes.unwrap_or(0);
    Ok(format!("Reminder set: '{}' in {} minutes", message, mins))
}
