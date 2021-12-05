#!/bin/sh

. ./test-lib.sh

test_expect_success 'remove untracked file fails' '
	! gnew remove test.txt 2>out &&
	grep "fatal: file not found" out
'

cat >expect <<\EOF &&
--- a/foo
+++ b/foo
@@ -1 +1 @@
-foo on main
+foo on branch1
--- /dev/null
+++ b/bar
@@ -0,0 +1 @@
+bar
EOF

test_expect_success 'diff between two branches' '
	test_commit "foo on main" foo &&
	gnew checkout -b branch1 &&
	test_commit "foo on branch1" foo &&
	test_commit bar bar &&
	gnew diff main branch1 >out &&
	diff expect out
'
