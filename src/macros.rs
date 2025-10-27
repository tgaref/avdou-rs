#[macro_export]
macro_rules! pattern {
    ( &[ $($pat:expr),+ $(,)? ] ) => {
        vec![$($pat.to_string()),+]
    };
}

#[macro_export]
macro_rules! compiler {
    ( $f:expr ) => {
        $f
    };
}

#[macro_export]
macro_rules! ctx {
    ( $ctx:expr ) => {
        $ctx
    };
}

#[macro_export]
macro_rules! template {
    ( $t:expr ) => {
        Some($t.to_string())
    };
}

#[macro_export]
macro_rules! route {
    ( $r:expr ) => {
        $r
    };
}

#[macro_export]
macro_rules! getmetadata {
    ( $r:expr ) => {
        $r
    };
}

#[macro_export]
macro_rules! rule {
    ($($args:tt)*) => {
        {
            let mut r = $crate::Rule::new();
            rule_items!(r; $($args)*);
            r
        }
    };
}

#[macro_export]
macro_rules! rule_items {
    ($r:ident; pattern!($pat:expr) ; $($rest:tt)*) => {{
	$r = $r.pattern($pat);
        rule_items!($r; $($rest)*);
    }};

    ($r:ident; compiler!($comp:expr) ; $($rest:tt)*) => {{
	$r = $r.compiler($comp);
        rule_items!($r; $($rest)*);
    }};

    ($r:ident; ctx!($ctx:expr) ; $($rest:tt)*) => {{
	$r = $r.context($ctx);
        rule_items!($r; $($rest)*);
    }};

    ($r:ident; template!($t:expr) ; $($rest:tt)*) => {{
	$r = $r.template($t);
        rule_items!($r; $($rest)*);
    }};

    ($r:ident; route!($rt:expr) ; $($rest:tt)*) => {{
        $r = $r.route($rt);
        rule_items!($r; $($rest)*);
    }};

    ($r:ident; getmetadata!($rt:expr) ; $($rest:tt)*) => {{
	$r = $r.getmetadata($rt);
        rule_items!($r; $($rest)*);
    }};

    // without ; at the end

    ($r:ident; pattern!($pat:expr) $($rest:tt)*) => {{
	$r = $r.pattern($pat);
        rule_items!($r; $($rest)*);
    }};

    ($r:ident; compiler!($comp:expr) $($rest:tt)*) => {{
	$r = $r.compiler($comp);
        rule_items!($r; $($rest)*);
    }};

    ($r:ident; ctx!($ctx:expr) $($rest:tt)*) => {{
	$r = $r.context($ctx);
        rule_items!($r; $($rest)*);
    }};

    ($r:ident; template!($t:expr) $($rest:tt)*) => {{
	$r = $r.template($t);
        rule_items!($r; $($rest)*);
    }};

    ($r:ident; route!($rt:expr) $($rest:tt)*) => {{
        $r = $r.route($rt);
        rule_items!($r; $($rest)*);
    }};

    ($r:ident; getmetadata!($rt:expr) $($rest:tt)*) => {{
	$r = $r.getmetadata($rt);
        rule_items!($r; $($rest)*);
    }};

    ($r:ident;) => {};
}

#[macro_export]
macro_rules! copy {
    ($($args:tt)*) => {{
        let mut c = $crate::Copy::new();
        copy_items!(c; $($args)*);
        c
    }};
}

#[macro_export]
macro_rules! copy_items {
    // --- pattern!(...) with semicolon ---
    ($c:ident; pattern!($pat:expr); $($rest:tt)*) => {{
        $c = $c.pattern($pat);
        copy_items!($c; $($rest)*);
    }};

    // --- route!(...) with semicolon ---
    ($c:ident; route!($rt:expr); $($rest:tt)*) => {{
        $c = $c.route($rt);
        copy_items!($c; $($rest)*);
    }};

    // --- pattern!(...) without semicolon ---
    ($c:ident; pattern!($pat:expr) $($rest:tt)*) => {{
        $c = $c.pattern($pat);
        copy_items!($c; $($rest)*);
    }};

    // --- route!(...) without semicolon ---
    ($c:ident; route!($rt:expr) $($rest:tt)*) => {{
        $c = $c.route($rt);
        copy_items!($c; $($rest)*);
    }};

    // --- end of recursion ---
    ($c:ident;) => {};
}

#[macro_export]
macro_rules! site {
    ( $($body:tt)* ) => {{
        let mut site = $crate::Site::new();
        site_items!(site; $($body)*);
        site
    }};
}

#[macro_export]
macro_rules! site_items {
    // --- site_dir!(...) ---
    ($s:ident; site_dir!($dir:expr) ; $($rest:tt)*) => {{
        $s = $s.site_dir($dir);
        site_items!($s; $($rest)*);
    }};

        ($s:ident; site_dir!($dir:expr) $($rest:tt)*) => {{
        $s = $s.site_dir($dir);
        site_items!($s; $($rest)*);
    }};

    // --- public_dir!(...) ---
    ($s:ident; public_dir!($dir:expr) ; $($rest:tt)*) => {{
        $s = $s.public_dir($dir);
        site_items!($s; $($rest)*);
    }};

    ($s:ident; public_dir!($dir:expr)  $($rest:tt)*) => {{
        $s = $s.public_dir($dir);
        site_items!($s; $($rest)*);
    }};

    // --- load_templates!(...) ---
    ($s:ident; load_templates!($pattern:expr) ; $($rest:tt)*) => {{
        $s = $s.load_templates($pattern);
        site_items!($s; $($rest)*);
    }};

    ($s:ident; load_templates!($pattern:expr) $($rest:tt)*) => {{
        $s = $s.load_templates($pattern);
        site_items!($s; $($rest)*);
    }};

    // --- rule!(...) ---
    ($s:ident; rule!($($args:tt)*) ; $($rest:tt)*) => {{
	let r = rule!($($args)*);
	$s = $s.rule(r);
	site_items!($s; $($rest)*);
    }};

    ($s:ident; rule!($($args:tt)*) $($rest:tt)*) => {{
	let r = rule!($($args)*);
	$s = $s.rule(r);
	site_items!($s; $($rest)*);
    }};

    // --- copy!(...) ---
    ($s:ident; copy!($($args:tt)*) ; $($rest:tt)*) => {{
	let c = copy!($($args)*);
	$s = $s.copy(c);
	site_items!($s; $($rest)*);
    }};

    ($s:ident; copy!($($args:tt)*) $($rest:tt)*) => {{
	let c = copy!($($args)*);
	$s = $s.copy(c);
	site_items!($s; $($rest)*);
    }};

    // --- Ignore unknown tokens gracefully (optional strict mode later) ---
    ($s:ident; $unknown:tt $($rest:tt)*) => {{
        compile_error!(concat!("Unknown item in site! macro: ", stringify!($unknown)));
        site_items!($s; $($rest)*);
    }};

    // --- Empty input (done) ---
    ($s:ident;) => {};
}

#[macro_export]
macro_rules! variables {
    // Base case: no entries
    () => {
        {
            use std::collections::HashMap;
            let map: HashMap<String, serde_yaml::Value> = HashMap::new();
            map
        }
    };

    // Main case: handle key = value, possibly followed by commas
    ( $( $key:expr => $val:expr ),+ $(,)? ) => {
        {
            use std::collections::HashMap;
            use serde_yaml::Value;
            let mut map: HashMap<String, Value> = HashMap::new();
            $(
                map.insert($key.to_string(), serde_yaml::to_value($val).unwrap());
            )+
            map
        }
    };
}

#[macro_export]
macro_rules! mine {
    ( $($inner:tt)* ) => {{
        let mut m = $crate::Mine::new();
        mine_items!(m; $($inner)*);
        m
    }};
}

#[macro_export]
macro_rules! mine_items {
    // pattern!([...])
    ($m:ident; pattern!($pat:expr); $($rest:tt)*) => {{
        $m = $m.pattern($pat);
        mine_items!($m; $($rest)*);
    }};

    // miner!(...)
    ($m:ident; miner!($func:expr); $($rest:tt)*) => {{
        $m = $m.miner($func);
        mine_items!($m; $($rest)*);
    }};

    // allow trailing semicolon
    ($m:ident;) => {};
}
