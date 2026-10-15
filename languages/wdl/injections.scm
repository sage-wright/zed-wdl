; Command blocks that contain a python heredoc (python/python3 <<EOF) → Python
((command_parts) @injection.content
 (#match? @injection.content "python[0-9.]*[ \t]+(-[ \t]*)?<<")
 (#set! injection.language "python"))

; Everything else → bash
((command_parts) @injection.content
 (#not-match? @injection.content "python[0-9.]*[ \t]+(-[ \t]*)?<<")
 (#set! injection.language "bash"))
