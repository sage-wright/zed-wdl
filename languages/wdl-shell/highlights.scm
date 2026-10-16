; Bash highlight query from tree-sitter-bash v0.25.1:
; https://github.com/tree-sitter/tree-sitter-bash/blob/v0.25.1/queries/highlights.scm
;
; The MIT License (MIT)
;
; Copyright (c) 2017 Max Brunsfeld
;
; Permission is hereby granted, free of charge, to any person obtaining a copy
; of this software and associated documentation files (the "Software"), to deal
; in the Software without restriction, including without limitation the rights
; to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
; copies of the Software, and to permit persons to whom the Software is
; furnished to do so, subject to the following conditions:
;
; The above copyright notice and this permission notice shall be included in all
; copies or substantial portions of the Software.
;
; THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
; IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
; FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
; AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
; LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
; OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
; SOFTWARE.
;
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
