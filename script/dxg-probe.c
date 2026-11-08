// DXCore 适配器枚举探针(无头文件依赖,裸 COM vtable 调用)。
// 用途:诊断 WSL GPU 半虚拟化是否暴露 D3D12 适配器(dzn/Vulkan-on-D3D12 的前提)。
//
// 编译:gcc dxg-probe.c -o dxg-probe -ldl
// 运行:LD_LIBRARY_PATH=/usr/lib/wsl/lib ./dxg-probe
//
// 判读:
//   DXCoreCreateAdapterFactory: 0x00000000   → dxcore 用户态正常
//   [GRAPHICS] adapter count: 0              → WSL 未暴露 D3D12 图形适配器(dzn 不可用)
//   [COMPUTE]  adapter count: 0              → 连 compute 通道也没有(本机实测结果)
//   适配器数 > 0                              → D3D12 可用,可继续 dzn 路线
//
// GUID 结构体布局必须与 Windows 二进制布局一致(Data1 为 4 字节),
// 用 unsigned long 会让 IID 字节错位 → E_NOINTERFACE(0x80004002)。

#include <stdint.h>
#include <stdio.h>
#include <dlfcn.h>

typedef long HRESULT;
typedef struct GUID_ { uint32_t a; uint16_t b, c; unsigned char d[8]; } G;

static const G IID_IDXCoreAdapterFactory =
    {0x78ee5945, 0xc36e, 0x4b13, {0xa6, 0x69, 0x00, 0x5d, 0xd1, 0x1c, 0x0f, 0x06}};
static const G IID_IDXCoreAdapterList =
    {0x526c7776, 0x40e9, 0x459b, {0xb7, 0x11, 0xf3, 0x2a, 0xd7, 0x6d, 0xfc, 0x28}};
static const G IID_IDXCoreAdapter =
    {0xf0db4c7f, 0xfe5a, 0x42a2, {0xbd, 0x62, 0xf2, 0xa6, 0xcf, 0x6f, 0xc8, 0x3e}};
static const G ATTR_D3D12_GRAPHICS =
    {0x0c9ece4d, 0x2f6e, 0x4f01, {0x8c, 0x96, 0xe8, 0x9e, 0x33, 0x1b, 0x47, 0xb1}};
static const G ATTR_D3D12_CORE_COMPUTE =
    {0x248e2800, 0xa793, 0x4724, {0xab, 0xaa, 0x23, 0xa6, 0xde, 0x1b, 0xe0, 0x90}};

/* vtable 槽位(含 QI/AddRef/Release),取自 DirectX-Headers 1.610 dxcore_interface.h */
#define VTBL(obj) (*(void ***)obj)

int main(void) {
    void *lib = dlopen("libdxcore.so", RTLD_NOW);
    if (!lib) {
        printf("dlopen libdxcore: %s\n", dlerror());
        return 1;
    }
    HRESULT (*create)(const G *, void **) =
        (HRESULT (*)(const G *, void **))dlsym(lib, "DXCoreCreateAdapterFactory");
    if (!create) {
        printf("no DXCoreCreateAdapterFactory export\n");
        return 1;
    }

    void *factory = NULL;
    HRESULT hr = create(&IID_IDXCoreAdapterFactory, &factory);
    printf("DXCoreCreateAdapterFactory: 0x%08lx\n", (unsigned long)(unsigned int)hr);
    if (hr != 0 || !factory) return 1;

    for (int which = 0; which < 2; which++) {
        const G *attr = which ? &ATTR_D3D12_CORE_COMPUTE : &ATTR_D3D12_GRAPHICS;
        void *list = NULL;
        HRESULT (*createList)(void *, unsigned long, const G *, const G *, void **) =
            (void *)(((void **)VTBL(factory))[3]);
        hr = createList(factory, 1, attr, &IID_IDXCoreAdapterList, &list);
        printf("[%s] CreateAdapterList: 0x%08lx\n",
               which ? "COMPUTE" : "GRAPHICS", (unsigned long)(unsigned int)hr);
        if (hr != 0 || !list) continue;
        unsigned long n = 0;
        HRESULT (*getCount)(void *, unsigned long *) =
            (void *)(((void **)VTBL(list))[6]);
        getCount(list, &n);
        printf("[%s] adapter count: %lu\n", which ? "COMPUTE" : "GRAPHICS", n);
        for (unsigned long i = 0; i < n; i++) {
            void *adapter = NULL;
            HRESULT (*getAdapter)(void *, unsigned long, const G *, void **) =
                (void *)(((void **)VTBL(list))[3]);
            if (getAdapter(list, i, &IID_IDXCoreAdapter, &adapter) != 0 || !adapter) continue;
            char desc[512] = {0};
            HRESULT (*getProp)(void *, unsigned long, unsigned long, void *) =
                (void *)(((void **)VTBL(adapter))[6]);
            HRESULT hrp = getProp(adapter, 2 /* DriverDescription */, sizeof(desc), desc);
            printf("  adapter %lu: hr=0x%08lx %s\n",
                   i, (unsigned long)(unsigned int)hrp, desc);
        }
    }
    return 0;
}
