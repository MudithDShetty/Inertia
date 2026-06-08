; PhysicsLang Tree-sitter grammar (source for tree-sitter generate)
; Used by IDE tooling and future incremental parser integration

(source_file
  (item)*)

(item
  (function_item)
  (qreg_decl)
  (let_decl)
  (extern_decl))

(function_item
  (attribute)*
  "fn" (identifier) "(" (parameter_list)? ")" ("->" (type_expr))?
  (block))

(qreg_decl
  "qreg" (identifier) "[" (number) "]")

(let_decl
  "let" (identifier) (":" (type_expr))? "=" (expression))

(attribute
  "@" (identifier) ("(" (string)? ")")?)

(block
  "{" (statement)* "}")

(statement
  (let_decl)
  (return_stmt)
  (expression))

(type_expr
  (identifier)
  (array_type))

(array_type
  (identifier) "[" (number) "]")

(expression
  (binary_expr)
  (unary_expr)
  (call_expr)
  (gate_expr)
  (quantity_literal)
  (identifier)
  (number))

(gate_expr
  (identifier) "(" (gate_args) ")")

(quantity_literal
  (number) (unit_expr))

(unit_expr
  (unit_factor) (("/" | "*") (unit_factor))*)

(unit_factor
  (identifier) ("^" (number))?)

(identifier) @identifier
(number) @number
"fn" @keyword
"let" @keyword
"return" @keyword
"qreg" @keyword
"@" @attribute
