# Error

The typed error every host-side failure becomes. The host speaks
`cartridge::{Error, Result}`; the SDK keeps its `Result<T, String>` and this
module is the boundary between the two.