; Comments (same as Zed's built-in bash)
((comment) @injection.content
 (#set! injection.language "comment"))

; ---------------------------------------------------------------------------
; 1. Heredoc fed to an interpreter:   python3 <<CODE ... CODE
;    Also matches /usr/bin/python3, Rscript - <<EOF, R --vanilla <<EOF, and
;    pipelines like `python - <<PY | sort`.
; ---------------------------------------------------------------------------
((redirected_statement
   body: (command name: (command_name) @_cmd)
   redirect: (heredoc_redirect (heredoc_body) @injection.content))
 (#match? @_cmd "(^|/)(python[0-9.]*|pypy[0-9]*)$")
 (#set! injection.language "python"))

((redirected_statement
   body: (command name: (command_name) @_cmd)
   redirect: (heredoc_redirect (heredoc_body) @injection.content))
 (#match? @_cmd "(^|/)(R|Rscript)$")
 (#set! injection.language "r"))

((redirected_statement
   body: (command name: (command_name) @_cmd)
   redirect: (heredoc_redirect (heredoc_body) @injection.content))
 (#match? @_cmd "(^|/)perl[0-9.]*$")
 (#set! injection.language "perl"))

((redirected_statement
   body: (command name: (command_name) @_cmd)
   redirect: (heredoc_redirect (heredoc_body) @injection.content))
 (#match? @_cmd "(^|/)ruby[0-9.]*$")
 (#set! injection.language "ruby"))

((redirected_statement
   body: (command name: (command_name) @_cmd)
   redirect: (heredoc_redirect (heredoc_body) @injection.content))
 (#match? @_cmd "(^|/)(node|nodejs)$")
 (#set! injection.language "javascript"))

((redirected_statement
   body: (command name: (command_name) @_cmd)
   redirect: (heredoc_redirect (heredoc_body) @injection.content))
 (#match? @_cmd "(^|/)julia$")
 (#set! injection.language "julia"))

((redirected_statement
   body: (command name: (command_name) @_cmd)
   redirect: (heredoc_redirect (heredoc_body) @injection.content))
 (#match? @_cmd "(^|/)(sqlite3|psql|duckdb|mysql)$")
 (#set! injection.language "sql"))

; ---------------------------------------------------------------------------
; 2. Heredoc written to a file: the file extension picks the language.
;    cat > plot.R <<EOF ... EOF        cat <<EOF > helper.py ... EOF
;    (Zed uses the text after the last "." of the captured name.)
; ---------------------------------------------------------------------------
(redirected_statement
  redirect: (file_redirect destination: (word) @injection.language)
  redirect: (heredoc_redirect (heredoc_body) @injection.content))

(redirected_statement
  redirect: (heredoc_redirect
    (file_redirect destination: (word) @injection.language)
    (heredoc_body) @injection.content))

; ---------------------------------------------------------------------------
; 3. Inline code in a double-quoted -c / -e argument:  python3 -c "print(1)"
;    (Single-quoted arguments can't be injected: Zed has no way to strip the
;    quotes from a bash raw_string.)
; ---------------------------------------------------------------------------
((command
   name: (command_name) @_cmd
   argument: (word) @_flag
   .
   argument: (string (string_content) @injection.content))
 (#match? @_cmd "(^|/)(python[0-9.]*|pypy[0-9]*)$")
 (#eq? @_flag "-c")
 (#set! injection.language "python"))

((command
   name: (command_name) @_cmd
   argument: (word) @_flag
   .
   argument: (string (string_content) @injection.content))
 (#match? @_cmd "(^|/)Rscript$")
 (#eq? @_flag "-e")
 (#set! injection.language "r"))

((command
   name: (command_name) @_cmd
   argument: (word) @_flag
   .
   argument: (string (string_content) @injection.content))
 (#match? @_cmd "(^|/)perl[0-9.]*$")
 (#eq? @_flag "-e")
 (#set! injection.language "perl"))

((command
   name: (command_name) @_cmd
   argument: (word) @_flag
   .
   argument: (string (string_content) @injection.content))
 (#match? @_cmd "(^|/)ruby[0-9.]*$")
 (#eq? @_flag "-e")
 (#set! injection.language "ruby"))

((command
   name: (command_name) @_cmd
   argument: (word) @_flag
   .
   argument: (string (string_content) @injection.content))
 (#match? @_cmd "(^|/)(node|nodejs)$")
 (#eq? @_flag "-e")
 (#set! injection.language "javascript"))
