window.SIDEBAR_ITEMS = {"enum":[["CodepointError","Error returned when an `u32` is not a valid unicode codepoint."],["FromStrError","Error returned when `Utf8Char::from_str()` or `Utf16Char::from_str()` fails."],["Utf16ArrayError","Error returned when an `[u16; 2]` doesn’t form a valid UTF-16 codepoint."],["Utf16PairError","Error returned by `Utf16CharDecoder` when it encounters an invalid sequence."],["Utf16SliceError","Error returned when a slice of `u16`s doesn’t start with valid UTF-16."],["Utf16TupleError","Error returned when one or two `u16`s are not valid UTF-16."],["Utf8ErrorKind","The types of errors that can occur when decoding a UTF-8 codepoint."]],"struct":[["EmptyStrError","Error returned by `Utf8Char::from_str_start()` and `Utf16Char::from_str_start()` when called with an empty string."],["NonAsciiError","Error returned by `Utf8Char::from_ascii()` for bytes that are not ASCII characters."],["NonBmpError","Error returned by `Utf16Char::from_bmp()` for units that are not a standalone codepoint."],["Utf16FirstUnitError","Error returned by `U16UtfExt::utf16_needs_extra_unit()` when called on an `u16` that’s a trailing surrogate."],["Utf8Error","Error returned when an invalid UTF-8 sequence is encountered."]]};