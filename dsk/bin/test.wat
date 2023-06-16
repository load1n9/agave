(module
  (type $t0 (func (param i32)))
  (type $t1 (func))
  (import "host" "hello" (func $host.hello (type $t0)))
  (func $hello (export "hello") (type $t1)
    (call $host.hello
      (i32.const 42))))
