# Summary

The supported desktop test path previously supplied an empty asset directory. SDK pack rejected it before compiling the host because index.html was absent. It now creates a minimal temporary entry document and cleans it on success and failure. No release UI is produced.
