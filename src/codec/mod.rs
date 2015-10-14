use super::Server;

pub enum Codec {
}

pub trait Ops {
    type Data;
    type Server: Server<Self::Data>;

    fn codec(&'static self) -> Codec;
    fn server(&'static self) -> Self::Server;
}
