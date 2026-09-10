const { isDeepStrictEqual } = require("node:util");
const fs = require("node:fs");

class Variant {
    constructor(name, payload) {
        this.name = name;
        this.payload = payload;
    }
}

class List {
    constructor(head, tail) {
        this.head = head;
        this.tail = tail;
    }

    static empty = new List(null, null);
}

let pure = (x) => () => x

let bind = (x, y) => () => y(x())()

let panic = (msg) => {
    process.stderr.write(msg);
    process.exit(1);
}

let trace = (msg) => (x) => {
    process.stderr.write(msg);
    return x;
}

let withFields = (x, fields) => {
    x = { ...x };
    Object.assign(x, fields);
    return x;
}

let stringHash = (str) => {
    let hash = 0;

    for (let i = 0; i < str.length; i++) {
        hash = ((hash << 5) - hash + str.charCodeAt(i)) | 0;
    }

    return hash >>> 0;
}

let numberHash = (x) => {
    x |= 0;
    x ^= x >>> 16;
    x = Math.imul(x, 0x85ebca6b);
    x ^= x >>> 13;
    x = Math.imul(x, 0xc2b2ae35);
    x ^= x >>> 16;
    return x >>> 0;
}

let stringIndexInto = (str, index) => {
  let n = 0;

  for (let i = 0; i < index; i++, n++) {
    const code = str.charCodeAt(i);

    if (
      code >= 0xD800 &&
      code <= 0xDBFF &&
      i + 1 < index
    ) {
      const next = str.charCodeAt(i + 1);

      if (next >= 0xDC00 && next <= 0xDFFF) {
        i++;
      }
    }
  }

  return n;
}

let stringIndexFrom = (str, n) => {
    let chars = 0;

    for (let i = 0; i < str.length; i++) {
        if (n === chars) {
            return i;
        }

        const code = str.charCodeAt(i);

        if (
            code >= 0xD800 &&
            code <= 0xDBFF &&
            i + 1 < str.length &&
            str.charCodeAt(i + 1) >= 0xDC00 &&
            str.charCodeAt(i + 1) <= 0xDFFF
        ) {
            i++;
        }

        chars++;
    }

    return str.length;
}

let stringFind = (haystack, needle) => {
    let index = haystack.indexOf(needle);

    if (index != -1) {
        let n = stringIndexInto(haystack, index);
        return new Variant("some", n);
    } else {
        return new Variant("none");
    }
}

let stringSplit = (str, n) => {
    let index = stringIndexFrom(str, n);
    return [str.slice(0, index), str.slice(index)]
}

let stringLength = (str) => {
    let n = 0;

    for (let i = 0; i < str.length; i++, n++) {
        const code = str.charCodeAt(i);

        if (
            code >= 0xD800 &&
            code <= 0xDBFF &&
            i + 1 < str.length &&
            str.charCodeAt(i + 1) >= 0xDC00 &&
            str.charCodeAt(i + 1) <= 0xDFFF
        ) {
            i++;
        }
    }

    return n;
}

let dynamic = (x) => {
    switch (typeof x) {

        case "function":
            return new Variant("lambda", null);

        case "number":
            return new Variant("real'", x);

        case "string":
            return new Variant("str'", x);

        case "bool":
            if (x) {
                return new Variant("true", new Variant("none"));
            } else {
                return new Variant("false", new Variant("none"));
            }

        case "object":
            switch (true) {
                case x instanceof Variant:
                    if (x.payload != null) {
                        return new Variant(
                            "variant",
                            [x.name, new Variant("some", dynamic(x.payload))]
                        );
                    } else {
                        return new Variant(
                            "variant",
                            [x.name, new Variant("none", null)]
                        );
                    }

                case x instanceof List:
                    let list = (xs) => {
                        if (xs.head != null) {
                            return new List(dynamic(xs.head), list(xs.tail));
                        } else {
                            return List.empty
                        }
                    }

                    return new Variant("list", list(x));

                case x[0] != null: {
                    let fields = List.empty;

                    for (let i = x.length - 1; i >= 0; i --) {
                        fields = new List(dynamic(x[i]), fields);
                    }

                    return new Variant("tuple", fields);
                }

                default: {
                    let fields = List.empty;

                    for (const [key, value] of Object.entries(x).toReversed()) {
                        fields = new List([key, dynamic(value)], fields);
                    }

                    return new Variant("record", fields);
                }
            }
    }
}

let extern = {
    "io::print": (s) => () => process.stdout.write(s),
    "fs::read-dir": (path) => () => {
        try {
            let files = List.empty;

            for (const entry of fs.readdirSync(path)) {
                files = new List(entry, files);
            }

            return new Variant("ok", files);
        } catch {
            return new Variant("err", new Variant("not-found"));
        }
    },
    "fs::read": (path) => () => {
        try {
            const data = fs.readFileSync(path, "utf8");
            return new Variant("ok", data);
        } catch {
            return new Variant("err", new Variant("not-found"));
        }
    },
    "fs::write": (path) => (data) => () => {
        try {
            const data = fs.writeFileSync(path, data, "utf8");
            return new Variant("ok", {});
        } catch {
            return new Variant("err", new Variant("not-found"));
        }
    },
    "fs::is-dir": (path) => () => {
        try {
            return fs.statSync(path).isDirectory();
        } catch {
            return false;
        }
    },
    "string::ansi-escape": "\x1b",
}
