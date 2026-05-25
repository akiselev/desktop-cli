//! macOS accessibility permission detection

#[cfg(target_os = "macos")]
pub fn check_accessibility_permission() -> bool {
    use accessibility_sys::AXIsProcessTrustedWithOptions;
    use core_foundation::base::{CFTypeRef, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;

    unsafe {
        // kAXTrustedCheckOptionPrompt is a CFStringRef in accessibility-sys 0.1
        let key = CFString::wrap_under_get_rule(accessibility_sys::kAXTrustedCheckOptionPrompt);
        let value = CFBoolean::true_value();

        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);

        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef() as _)
    }
}

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility_permission() -> bool {
    false
}
