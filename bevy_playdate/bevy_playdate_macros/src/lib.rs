use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn init_app(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_body = &input_fn.block;
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;

    let expanded = quote! {
        #[unsafe(no_mangle)]
        fn event_handler(_api: core::ptr::NonNull<pd::sys::ffi::PlaydateAPI>, event: pd::sys::ffi::PDSystemEvent, _: u32) -> pd::sys::EventLoopCtrl {
            match event {
                pd::sys::ffi::PDSystemEvent::kEventInit => {}
                _ => return pd::sys::EventLoopCtrl::Continue,
            }

            let mut app = init_app();

            pd::system::System::Default().set_update_callback_boxed(
                move |_| {
                    app.update();
                    pd::system::update::UpdateCtrl::Continue
                },
                (),
            );

            pd::sys::EventLoopCtrl::Continue
        }

        #fn_vis #fn_sig #fn_body
    };

    TokenStream::from(expanded)
}
