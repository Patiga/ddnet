use ddnet_base::StrRef;

#[cxx::bridge]
mod ffi {
    extern "C++" {
        include!("base/rust.h");

        type StrRef<'a> = ddnet_base::StrRef<'a>;
    }
    extern "Rust" {
        fn BackendWgpuGreetings(names: &[StrRef<'_>]);
    }
}

/// Example for a Rust function callable from C++.
///
/// Prints a greeting from the wgpu backend module to stdout, mentioning the
/// passed names.
#[allow(non_snake_case)]
fn BackendWgpuGreetings(names: &[StrRef<'_>]) {
    if names.is_empty() {
        println!("Hello!");
    } else {
        let names: Vec<_> = names.into_iter().map(StrRef::to_str).collect();
        println!("Hello, {}!", names.join(", "));
    }
}
