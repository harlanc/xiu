// trait handshake {
//     // Static method signature; `Self` refers to the implementor type.
//     fn new(name: &'static str) -> Self;

//     // Instance method signatures; these will return a string.
//     fn name(&self) -> &'static str;
//     fn noise(&self) -> &'static str;

//     // Traits can provide default method definitions.
//     fn talk(&self) {
//         println!("{} says {}", self.name(), self.noise());
//     }

//     read_c0c1() ->      err
// }

struct SimpleHandshake {}

struct ComplexHandshake {}

impl SimpleHandshake {
    fn new() -> SimpleHandshake {
        SimpleHandshake {}
    }

    fn handshake_with_client() {}

    fn handshake_with_server() {}
}
