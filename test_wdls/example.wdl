version 1.2

## Example file for checking the extension's features:
##   * WDL 1.2 syntax (env inputs, else if, requirements, hints)
##   * outline: structs, workflow, tasks and calls appear in the outline panel
##   * embedded-language highlighting inside command blocks (see task EmbeddedLanguages)

import "local.wdl" as local alias Foo as Bar
import "https://example.com/remote.wdl" as remote alias Baz as Blorf

struct Example1 {
    Float f
    Map[String, Int] m
}

struct Example2 {
    String s
    Int? i
    Array[File?]+ a
    Example1 e
    Object o
    Pair[String, String] p
}

workflow Workflow1 {
    input {
        String s
        Int i = ceil(0)
        Example2? ex
    }

    Float f = i + 1.0
    Array[File] file_array = if defined(ex) then select_all(select_first([ex]).a) else []
    String size_class = if i > 100 then "large" else if i > 10 then "medium" else "small"

    call local.foo
    call local.foo as bar {}
    call local.baz {
        input:
    }
    call local.qux {
        input:
            x = 1,
            y = false
    }

    if (1 > 2) {
        scatter (file in file_array) {
            call remote.waldo {
                input: file
            }
        }
    }

    call Task1 { name = s }
    call EmbeddedLanguages { samplename = s, reads = file_array }

    output {
        File name_file = Task1.name_file
        File report = EmbeddedLanguages.report
    }

    meta {
        description: "Test workflow"
        test: true
        size: 10
        numbers: [1, 2, 3]
        keywords: {
            a: 1.0,
            b: -1
        }
        x: null
    }
}

task Task1 {
    parameter_meta {
        name: "name of the file"
    }

    input {
        String name
        Example2? ex
        String docker_image = "debian:bookworm-slim"
        env String GREETING = "hello"
    }

    # Plain bash: shell keywords, strings and variables are highlighted as bash,
    # while ~{...} placeholders keep WDL highlighting.
    command <<<
        set -euxo pipefail

        echo "$GREETING, name: ~{name}" \
            | cat

        if [[ "test" =~ \.fastq ]]; then
            echo "yes"
        else
            echo "no"
        fi

        echo "~{sep(" ", ['John', 'Doe'])}"
        echo "~{if defined(ex) then "yes" else "no"}"

        # A plain heredoc that is not fed to an interpreter stays a bash string.
        cat <<EOF | tee notes.txt
        just some text for ~{name}
        EOF
    >>>

    output {
        File name_file = stdout()
    }

    requirements {
        container: docker_image
        cpu: 1
        memory: "2 GiB"
    }

    hints {
        disks: "10 GiB"
    }

    meta {
        description: "write name to file"
    }
}

task EmbeddedLanguages {
    input {
        String samplename
        Array[File] reads
        Int cpu = 2
    }

    command <<<
        set -euo pipefail

        # Python heredoc (python, python3, pypy and full paths like /usr/bin/python3)
        python3 <<CODE
        import json
        import sys

        reads = "~{sep(",", reads)}".split(",")
        summary = {"sample": "~{samplename}", "n_reads": len(reads)}
        with open("summary.json", "w") as fh:
            json.dump(summary, fh, indent=2)
        print(f"wrote summary for {summary['sample']}", file=sys.stderr)
        CODE

        # Interpreter heredocs work inside pipelines too
        /usr/bin/python3 - <<'PY' | sort > sorted.txt
        for name in ["b", "a", "c"]:
            print(name)
        PY

        # R heredocs (Rscript or R)
        Rscript - <<'RSCRIPT'
        summary <- jsonlite::fromJSON("summary.json")
        write.csv(data.frame(sample = summary$sample, n = summary$n_reads), "summary.csv")
        RSCRIPT

        R --vanilla --no-save <<'RCODE'
        x <- seq(1, 10)
        print(mean(x))
        RCODE

        # Perl, JavaScript (node), Julia and SQL heredocs
        perl <<'PERL'
        my @fields = split(/\t/, "a\tb\tc");
        print scalar(@fields), "\n";
        PERL

        node <<'JS'
        const counts = { a: 1, b: 2 };
        console.log(JSON.stringify(counts));
        JS

        julia <<'JULIA'
        println(sum([1, 2, 3]))
        JULIA

        sqlite3 results.db <<'SQL'
        CREATE TABLE IF NOT EXISTS runs (sample TEXT, n_reads INTEGER);
        SELECT COUNT(*) FROM runs;
        SQL

        # A heredoc written to a file is highlighted by the file's extension
        cat > plot.R <<'EOF'
        png("plot.png")
        plot(1:10, main = "example")
        dev.off()
        EOF

        cat <<'EOF' > config.json
        {"threads": ~{cpu}, "sample": "~{samplename}"}
        EOF

        # Inline code in double-quoted -c / -e arguments
        python3 -c "import sys; print(sys.version)"
        Rscript -e "cat(R.version.string)"
        perl -e "print 42"

        # Single-quoted inline code is NOT highlighted (known limitation)
        Rscript -e 'cat("not highlighted")'

        echo "report for ~{samplename}" > report.txt
    >>>

    output {
        File report = "report.txt"
        File summary_json = "summary.json"
    }

    requirements {
        container: "python:3.12-slim"
        cpu: cpu
    }
}
