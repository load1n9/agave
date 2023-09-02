(module
  (type (;0;) (func (param i32)))
  (type (;1;) (func))
  (import "temp" "hello" (func (;0;) (type 0)))
  (func $hello (type 1)
    i32.const 42
    call 0)
  (export "_start" (func 1))
  (export "update" (func 1))
  )
