use ::serde::de::DeserializeOwned;
use ::serde::Serialize;
use ::std::fmt::Debug;

pub trait Method: Serialize+Clone {
    const TYPE: &'static str;
    type Response: DeserializeOwned+Debug;

    fn tag(self) -> MethodType<Self>
        where Self: ::std::marker::Sized {
        MethodType {
            type_: Self::TYPE,
            payload: self,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct MethodType<T: Method> {
    #[serde(rename="@type")]
    pub type_: &'static str,
    #[serde(flatten)]
    pub payload: T,
}

include!(concat!(env!("OUT_DIR"), "/td_api.rs"));
