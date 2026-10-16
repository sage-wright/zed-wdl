; Bash highlight query from the Zed editor:
; https://github.com/zed-industries/zed/blob/main/crates/grammars/src/bash/highlights.scm
; Copyright Zed Industries, Inc.
;
; This file is licensed under the GNU General Public License, version 3 or
; (at your option) any later version. See LICENSE-GPL in this directory.
; The rest of this extension is licensed under BSD-3-Clause (see LICENSE).

[
  (string)
  (raw_string)
  (heredoc_body)
  (heredoc_start)
  (heredoc_end)
  (ansi_c_string)
  (word)
] @string

(variable_name) @variable

[
  "export"
  "function"
  "unset"
  "local"
  "declare"
] @keyword

[
  "case"
  "do"
  "done"
  "elif"
  "else"
  "esac"
  "fi"
  "for"
  "if"
  "in"
  "select"
  "then"
  "until"
  "while"
] @keyword.control

(comment) @comment

; Shebang
((program
  .
  (comment) @keyword.directive)
  (#match? @keyword.directive "^#![ \t]*/"))

(function_definition
  name: (word) @function)

(command_name
  (word) @function)

[
  (file_descriptor)
  (number)
] @number

(regex) @string.regex

[
  (command_substitution)
  (process_substitution)
  (expansion)
] @embedded

[
  "$"
  "&&"
  "||"
  ">"
  "<<"
  ">>"
  ">&"
  ">&-"
  "<"
  "|"
  ":"
  "//"
  "/"
  "%"
  "%%"
  "#"
  "##"
  "+="
  "-="
  "*="
  "/="
  "%="
  "**="
  "<<="
  ">>="
  "&="
  "^="
  "|="
  "="
  "=~"
  "=="
  "!="
  "-o"
  "-a"
  "^"
  "&"
  "<="
  ">="
  "+"
  "-"
  "*"
  "**"
  "!"
  "++"
  "--"
  "~"
  "?"
] @operator

(test_operator) @keyword.operator

";" @punctuation.delimiter

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

(test_command
  [
    "[["
    "]]"
  ] @punctuation.bracket)

(compound_statement
  [
    "(("
    "))"
  ] @punctuation.bracket)

(simple_expansion
  "$" @punctuation.special)

(expansion
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

(command_substitution
  "$(" @punctuation.special
  ")" @punctuation.special)

(arithmetic_expansion
  [
    "$(("
    "$["
    "))"
    "]"
  ] @punctuation.special)

((command
  (_) @constant)
  (#match? @constant "^-"))

(case_item
  value: (_) @string.regex)

(special_variable_name) @variable.special

; --- Additions for the WDL extension (BSD-3-Clause, see LICENSE) ---
;
; --- Mark the delimiters of heredocs that hold another language ---
((redirected_statement
   body: (command name: (command_name) @_cmd)
   redirect: (heredoc_redirect
     (heredoc_start) @embedded.delimiter
     (heredoc_end) @embedded.delimiter))
 (#match? @_cmd "(^|/)(python[0-9.]*|pypy[0-9]*|R|Rscript|perl[0-9.]*|ruby[0-9.]*|node|nodejs|julia|sqlite3|psql|duckdb|mysql)$"))

(redirected_statement
  redirect: (file_redirect)
  redirect: (heredoc_redirect
    (heredoc_start) @embedded.delimiter
    (heredoc_end) @embedded.delimiter))

(heredoc_redirect
  (heredoc_start) @embedded.delimiter
  redirect: (file_redirect)
  (heredoc_end) @embedded.delimiter)
