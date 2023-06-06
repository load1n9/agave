(module
  (type $t0 (func (param i32)))
  (type $t1 (func))
  (import "wasi_unstable" "proc_exit" (func $proc_exit (type $t0)))
  (func $hello (export "hello") (type $t1)
    (call $proc_exit
      (i32.const 1))))
