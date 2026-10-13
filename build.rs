//! 构建脚本：在 Windows 上把 assets/XO.ico 嵌入可执行文件作为程序图标。

fn main() {
    println!("cargo:rerun-if-changed=assets/XO.ico");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(target_os = "windows")]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/XO.ico");
        resource.set("FileDescription", "XO ICO Generator");
        resource.set("ProductName", "XO ICO Generator");
        // 资源编译器（rc.exe）缺失时降级为警告，不阻断构建
        if let Err(err) = resource.compile() {
            println!("cargo:warning=嵌入程序图标失败（将使用默认图标）: {err}");
        }
    }
}
