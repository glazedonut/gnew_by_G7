#!/bin/sh

. ./test-lib.sh

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

test_expect_success 'remove missing file works' '
	rm bar &&
	gnew remove bar
'
