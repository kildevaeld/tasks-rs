use crate::{AssetRequest, AssetResponse, Error};
use tasks::{task, Task, TaskExt};

mod any;

pub use any::*;

// pub fn file() -> impl Task<AssetRequest, Output = (AssetRequest, (File,)), Error = Error> + Copy {
//     task!(|req| async {
//          match req. {

//          }
//     })
// }
