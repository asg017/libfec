import gleeunit
import libfec

pub fn main() -> Nil {
  gleeunit.main()
}

pub fn native_version_test() {
  assert libfec.native_version() == "0.0.32"
}
