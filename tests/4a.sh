#!/bin/sh

. ./test-lib.sh

test_expect_success 'remove removes files from the tracklist' '
	test_commit "remove me" file &&
	gnew remove file &&
	gnew status >out &&
	grep "R file" out
'

test_expect_success 'diff shows removed file' '
	cat >expect <<-\EOF &&
	--- a/file
	+++ /dev/null
	@@ -1 +0,0 @@
	-remove me
	EOF
	gnew diff >out &&
	diff expect out
'
