use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote, Data, DeriveInput, Fields, GenericParam, Generics};

#[proc_macro_derive(CdrEncodingSize)]
pub fn derive_cdr_encoding_size(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let name = input.ident;

    // Add a bound `T: HeapSize` to every type parameter T.
    let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Generate an expression to sum up the heap size of each field.
    let sum = cdr_size_sum(&input.data);

    let expanded = quote! {
        // The generated impl.
        impl #impl_generics cdr_encoding_size::CdrEncodingSize for #name #ty_generics #where_clause {
            fn cdr_encoding_max_size() -> cdr_encoding_size::CdrEncodingMaxSize {
                #sum
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

// Add a bound `T: HeapSize` to every type parameter T.
fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(cdr_encoding_size::CdrEncodingSize));
        }
    }
    generics
}

// Generate an expression to sum up the size of each field.
fn cdr_size_sum(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    // Expands to an expression like
                    //
                    //     0 + self.x.heap_size() + self.y.heap_size() + self.z.heap_size()
                    //
                    // but using fully qualified function call syntax.
                    //
                    // We take some care to use the span of each `syn::Field` as
                    // the span of the corresponding `heap_size_of_children`
                    // call. This way if one of the field types does not
                    // implement `HeapSize` then the compiler's error message
                    // underlines which field it is. An example is shown in the
                    // readme of the parent directory.
                    let recurse = fields.named.iter().map(|f| {
                        let ty = &f.ty;
                        quote_spanned! {f.span()=>
                            <#ty>::cdr_encoding_max_size()
                        }
                    });
                    quote! {
                        cdr_encoding_size::CdrEncodingMaxSize::Bytes(0) #(+ #recurse)*
                    }
                }
                Fields::Unnamed(ref fields) => {
                    // Expands to an expression like
                    //
                    //     0 + self.0.heap_size() + self.1.heap_size() + self.2.heap_size()
                    let recurse = fields.unnamed.iter().enumerate().map(|(_i, f)| {
                        let ty = &f.ty;
                        //let index = Index::from(i);
                        quote_spanned! {f.span()=>
                            <#ty>::cdr_encoding_max_size()
                        }
                    });
                    quote! {
                        cdr_encoding_size::CdrEncodingMaxSize::Bytes(0) #(+ #recurse)*
                    }
                }
                Fields::Unit => {
                    // Unit structs cannot own more than 0 bytes of heap memory.
                    quote!(cdr_encoding_size::CdrEncodingMaxSize::Bytes(0))
                }
            }
        }
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}