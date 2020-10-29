/// Short form to compose Error values.
///
/// Here are few possible ways:
///
/// ```ignore
/// use crate::Error;
/// err_at!(Error::Invalid(String::default(), "bad argument"));
/// ```
///
/// ```ignore
/// use crate::Error;
/// err_at!(Invalid, msg: format!("bad argument"));
/// ```
///
/// ```ignore
/// use crate::Error;
/// err_at!(Invalid, std::io::read(buf));
/// ```
///
/// ```ignore
/// use crate::Error;
/// err_at!(Invalid, std::fs::read(file_path), format!("read failed"));
/// ```
///
#[macro_export]
macro_rules! err_at {
    ($v:ident, msg: $($arg:expr),+) => {{
        use log::error;

        let prefix = format!("{}:{}", file!(), line!());
        let err = Error::$v(prefix, format!($($arg),+));

        error!("{}", err);
        Err(err)
    }};
    ($v:ident, $e:expr) => {{
        use log::error;

        match $e {
            Ok(val) => Ok(val),
            Err(err) => {
                let prefix = format!("{}:{}", file!(), line!());
                let err = Error::$v(prefix, format!("{}", err));

                error!("{}", err);
                Err(err)
            }
        }
    }};
    ($v:ident, $e:expr, $($arg:expr),+) => {{
        use log::error;

        match $e {
            Ok(val) => Ok(val),
            Err(err) => {
                let prefix = format!("{}:{}", file!(), line!());
                let msg = format!($($arg),+);
                let err = Error::$v(prefix, format!("{} {}", err, msg));

                error!("{}", err);

                Err(err)
            }
        }
    }};
}
