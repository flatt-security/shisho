FROM test
FROM test as alias1
FROM foo
FROM foo as alias2

FROM test:tag
FROM test:tag as alias3
FROM foo:tag
FROM foo:tag as alias4

FROM test@hash
FROM test@hash as alias5
FROM foo@hash
FROM foo@hash as alias6

FROM test:tag@hash
FROM test:tag@hash as alias7
FROM foo:tag@hash
FROM foo:tag@hash as alias8