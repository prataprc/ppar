Release Checklist
=================

* Cleanup TODO items and TODO.md.
* Cleanup any println!(), panic!(), unreachable!(), unimplemented!() macros.
* Cleanup unwanted fmt::Debug and fmt::Display.
* Check for unwrap()/expect() calls and "as" type cast.
* README
  * Link to rust-doc.
  * Short description.
  * Useful links.
  * Contribution guidelines.
* Make build, prepare, flamegraph.
* Documentation Review.
* Bump up the version:
  * __major__: backward incompatible API changes.
  * __minor__: backward compatible API Changes.
  * __patch__: bug fixes.
* Create a git-tag for the new version.
* Cargo publish the new version.
