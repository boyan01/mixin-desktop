const APP_VERSION: &str = env!("MIXIN_APP_VERSION");
const APP_BUILD_NUMBER: &str = env!("MIXIN_APP_BUILD_NUMBER");

pub fn provisioning_app_version() -> String {
    format!("{APP_VERSION}({APP_BUILD_NUMBER})")
}

pub fn generate_user_agent() -> String {
    format!(
        "Mixin/{APP_VERSION} (Flutter {}; {})",
        system_and_version(),
        locale_name(),
    )
}

pub fn provisioning_platform_version() -> String {
    #[cfg(target_os = "macos")]
    unsafe {
        use objc2_foundation::NSProcessInfo;

        return NSProcessInfo::processInfo()
            .operatingSystemVersionString()
            .to_string();
    }

    #[allow(unreachable_code)]
    system_and_version()
}

fn system_and_version() -> String {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("sw_vers").output();
        if let Ok(output) = output {
            let values = String::from_utf8_lossy(&output.stdout);
            let product_name = plist_value(&values, "ProductName");
            let product_version = plist_value(&values, "ProductVersion");
            let build_version = plist_value(&values, "BuildVersion");
            if let (Some(name), Some(version), Some(build)) =
                (product_name, product_version, build_version)
            {
                return format!("{name} {version}({build})");
            }
        }
    }

    format!("{}({})", std::env::consts::OS, os_version())
}

#[cfg(target_os = "macos")]
fn plist_value<'a>(values: &'a str, key: &str) -> Option<&'a str> {
    values.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        (name.trim() == key).then_some(value.trim())
    })
}

fn os_version() -> String {
    std::process::Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn locale_name() -> String {
    std::env::var("LANG")
        .ok()
        .and_then(|value| value.split('.').next().map(str::to_string))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "en_US".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_matches_flutter_format() {
        assert!(generate_user_agent().starts_with("Mixin/5.1.2 (Flutter "));
        assert_eq!(provisioning_app_version(), "5.1.2(557)");
    }
}
