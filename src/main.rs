mod scroll_accel;

fn main() {
    // 每个功能对应一个独立模块，后续新增功能时在此声明 mod 并调用即可。
    scroll_accel::run();
}
