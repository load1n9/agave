cfg_if::cfg_if! {
    if #[cfg(target_arch = "x86_64")] {
        mod x86_64;
        pub(crate) use self::x86_64::*;
    }
    // else if #[cfg(target_arch = "aarch64")] {
    //     mod aarch64;
    //     pub(crate) use self::aarch64::*;
    // } else {
    //     mod unsupported;
    //     pub(crate) use self::unsupported::*;
    // }
}
