//! macOS accessibility permission detection

#[cfg(target_os = "macos")]
pub fn check_accessibility_permission() -> bool {
    use accessibility_sys::{
        kAXTrustedCheckOptionPrompt, AXIsProcessTrustedWithOptions, CFDictionaryCreate,
        CFDictionaryRef,
    };
    use core_foundation::base::{kCFBooleanTrue, CFTypeRef, TCFType};
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;
    use std::ptr;

    unsafe {
        let key = CFString::new(kAXTrustedCheckOptionPrompt).as_CFTypeRef();
        let value = kCFBooleanTrue as CFTypeRef;

        let keys: [CFTypeRef; 1] = [key];
        let values: [CFTypeRef; 1] = [value];

        let options: CFDictionaryRef = CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            ptr::null(),
            ptr::null(),
        );

        let trusted = AXIsProcessTrustedWithOptions(options as _);

        if !options.is_null() {
            let options_dict = CFDictionary::wrap_under_create_rule(options);
            drop(options_dict);
        }

        trusted != 0
    }
}

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility_permission() -> bool {
    false
}
