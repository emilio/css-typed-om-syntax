use super::{Impl, Component, ComponentName, Multiplier};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DataType {
    Length,
    Number,
    Percentage,
    LengthPercentage,
    Color,
    Image,
    Url,
    Integer,
    Angle,
    Time,
    Resolution,
    TransformFunction,
    TransformList,
    CustomIdent,
}

impl DataType {
    pub fn unpremultiply<I: Impl<DataType = Self>>(&self) -> Option<Component<I>> {
        match *self {
            DataType::TransformList => Some(Component {
                name: ComponentName::DataType(DataType::TransformFunction),
                multiplier: Some(Multiplier::Space),
            }),
            _ => None,
        }
    }

    pub fn from_str(ty: &str) -> Option<Self> {
        Some(match ty.as_bytes() {
            b"length" => DataType::Length,
            b"number" => DataType::Number,
            b"percentage" => DataType::Percentage,
            b"length-percentage" => DataType::LengthPercentage,
            b"color" => DataType::Color,
            b"image" => DataType::Image,
            b"url" => DataType::Url,
            b"integer" => DataType::Integer,
            b"angle" => DataType::Angle,
            b"time" => DataType::Time,
            b"resolution" => DataType::Resolution,
            b"transform-function" => DataType::TransformFunction,
            b"custom-ident" => DataType::CustomIdent,
            b"transform-list" => DataType::TransformList,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomIdent(Box<str>);

impl CustomIdent {
    pub fn from_ident(ident: &str) -> Option<Self> {
        if ident.eq_ignore_ascii_case("inherit") ||
            ident.eq_ignore_ascii_case("reset") ||
            ident.eq_ignore_ascii_case("revert") ||
            ident.eq_ignore_ascii_case("unset") ||
            ident.eq_ignore_ascii_case("default") {
            return None;
        }
        Some(CustomIdent(ident.to_owned().into_boxed_str()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DefaultImpl;

impl Impl for DefaultImpl {
    type CustomIdent = CustomIdent;
    type DataType = DataType;

    fn data_type_name_from_str(ty: &str) -> Option<DataType> {
        DataType::from_str(ty)
    }

    fn custom_ident_from_ident(ident: &str) -> Option<CustomIdent> {
        CustomIdent::from_ident(ident)
    }

    fn unpremultiply_data_type(ty: &DataType) -> Option<Component<Self>> {
        ty.unpremultiply()
    }
}
