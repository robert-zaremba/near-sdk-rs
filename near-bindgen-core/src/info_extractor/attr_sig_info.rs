use crate::info_extractor::arg_info::{ArgInfo, BindgenArgType};
use crate::info_extractor::serializer_attr::SerializerAttr;
use crate::info_extractor::SerializerType;
use quote::ToTokens;
use syn::export::Span;
use syn::spanned::Spanned;
use syn::{Attribute, Error, FnArg, Ident, Receiver, ReturnType, Signature};

/// Information extracted from method attributes and signature.
pub struct AttrSigInfo {
    /// The name of the method.
    pub ident: Ident,
    /// Attributes not related to bindgen.
    pub non_bindgen_attrs: Vec<Attribute>,
    /// All arguments of the method.
    pub args: Vec<ArgInfo>,
    /// Whether method can be used as initializer.
    pub is_init: bool,
    /// The serializer that we use for `env::input()`.
    pub input_serializer: SerializerType,
    /// The serializer that we use for the return type.
    pub result_serializer: SerializerType,
    /// The receiver, like `mut self`, `self`, `&mut self`, `&self`, or `None`.
    pub receiver: Option<Receiver>,
    /// What this function returns.
    pub returns: ReturnType,
    /// The original code of the method.
    pub original_sig: Signature,
}

impl AttrSigInfo {
    /// Process the method and extract information important for near-bindgen.
    pub fn new(original_attrs: Vec<Attribute>, original_sig: Signature) -> syn::Result<Self> {
        if original_sig.asyncness.is_some() {
            return Err(Error::new(
                original_sig.span(),
                "Contract API is not allowed to be async.",
            ));
        }
        if original_sig.abi.is_some() {
            return Err(Error::new(
                original_sig.span(),
                "Contract API is not allowed to have binary interface.",
            ));
        }
        if !original_sig.generics.params.is_empty() {
            return Err(Error::new(
                original_sig.span(),
                "Contract API is not allowed to have generics.",
            ));
        }
        if original_sig.variadic.is_some() {
            return Err(Error::new(
                original_sig.span(),
                "Contract API is not allowed to have variadic arguments.",
            ));
        }

        let ident = original_sig.ident.clone();
        let mut non_bindgen_attrs = vec![];
        let mut args = vec![];
        let mut is_init = false;
        // By the default we serialize the result with JSON.
        let mut result_serializer = SerializerType::JSON;
        for attr in &original_attrs {
            let attr_str = attr.path.to_token_stream().to_string();
            match attr_str.as_str() {
                "init" => {
                    is_init = true;
                }
                "result_serializer" => {
                    let serializer: SerializerAttr = syn::parse2(attr.tokens.clone())?;
                    result_serializer = serializer.serializer_type;
                }
                _ => non_bindgen_attrs.push((*attr).clone()),
            }
        }

        let returns = original_sig.output.clone();
        let mut receiver = None;
        for fn_arg in &original_sig.inputs {
            match fn_arg {
                FnArg::Receiver(r) => receiver = Some((*r).clone()),
                FnArg::Typed(pat_typed) => {
                    args.push(ArgInfo::new((*pat_typed).clone())?);
                }
            }
        }

        let mut result = Self {
            ident,
            non_bindgen_attrs,
            args,
            input_serializer: SerializerType::JSON,
            is_init,
            result_serializer,
            receiver,
            returns,
            original_sig,
        };

        let input_serializer =
            if result.input_args().all(|arg: &ArgInfo| arg.serializer_ty == SerializerType::JSON) {
                SerializerType::JSON
            } else if result.input_args().all(|arg| arg.serializer_ty == SerializerType::Borsh) {
                SerializerType::Borsh
            } else {
                return Err(Error::new(
                    Span::call_site(),
                    format!("Input arguments should be all of the same serialization type."),
                ));
            };
        result.input_serializer = input_serializer;
        Ok(result)
    }

    /// Only get args that correspond to `env::input()`.
    pub fn input_args(&self) -> impl Iterator<Item = &ArgInfo> {
        self.args.iter().filter(|arg| match arg.bindgen_ty {
            BindgenArgType::Regular => true,
            _ => false,
        })
    }
}