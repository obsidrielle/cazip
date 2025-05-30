#!/bin/bash

# 测试 cazip 程序的 ZIP 压缩/解压功能的脚本
# 使用方法: ./test_zip.sh [cazip路径]

# 设置颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# 设置 cazip 路径
CAZIP=${1:-"./cazip"}

# 检查 cazip 是否存在
if [ ! -f "$CAZIP" ]; then
    echo -e "${RED}错误: cazip 程序 ($CAZIP) 不存在${NC}"
    echo "请提供正确的路径: $0 /path/to/cazip"
    exit 1
fi

# 创建测试目录
TEST_DIR="cazip_zip_test_$(date +%s)"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

echo -e "${YELLOW}[INFO]${NC} 测试目录: $PWD"
echo -e "${YELLOW}[INFO]${NC} 使用的 cazip: $CAZIP"

# 初始化测试计数器
TESTS_TOTAL=0
TESTS_PASSED=0

# 测试函数
run_test() {
    local test_name=$1
    local cmd=$2
    local validation=$3

    TESTS_TOTAL=$((TESTS_TOTAL + 1))

    echo -e "\n${YELLOW}[TEST ${TESTS_TOTAL}]${NC} $test_name"
    echo -e "${YELLOW}[CMD]${NC} $cmd"

    # 执行命令
    eval "$cmd"
    local cmd_status=$?

    # 执行验证
    eval "$validation"
    local val_status=$?

    if [ $cmd_status -eq 0 ] && [ $val_status -eq 0 ]; then
        echo -e "${GREEN}[PASSED]${NC} $test_name"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}[FAILED]${NC} $test_name"
        if [ $cmd_status -ne 0 ]; then
            echo -e "${RED}[ERROR]${NC} 命令执行失败 (状态码: $cmd_status)"
        fi
        if [ $val_status -ne 0 ]; then
            echo -e "${RED}[ERROR]${NC} 验证失败 (状态码: $val_status)"
        fi
    fi
}

# 创建测试数据
echo -e "\n${YELLOW}[INFO]${NC} 创建测试数据..."

# 创建单个测试文件
echo "这是一个测试文件内容。" > single_file.txt

# 创建测试目录结构
mkdir -p test_directory/subdir1
mkdir -p test_directory/subdir2
echo "文件1内容" > test_directory/file1.txt
echo "文件2内容" > test_directory/subdir1/file2.txt
echo "文件3内容" > test_directory/subdir2/file3.txt

# 创建较大的测试文件 (1MB)
dd if=/dev/urandom of=large_file.bin bs=1M count=1 2>/dev/null

# 创建可执行文件，测试权限保留
echo "#!/bin/bash" > executable.sh
echo "echo 'Hello from executable'" >> executable.sh
chmod +x executable.sh

echo -e "${GREEN}[SUCCESS]${NC} 测试数据准备完成"

# ====== 测试用例开始 ======

# 测试用例1: 单文件压缩 (内部实现)
run_test "单文件压缩 (内部实现)" \
    "$CAZIP compress -f zip single_file.zip single_file.txt" \
    "[ -f single_file.zip ] && [ -s single_file.zip ]"

# 测试用例2: 单文件解压 (内部实现)
mkdir -p extract_single
run_test "单文件解压 (内部实现)" \
    "$CAZIP extract -f zip extract_single single_file.zip" \
    "[ -f extract_single/single_file.txt ] && diff single_file.txt extract_single/single_file.txt"

# 测试用例3: 目录压缩 (内部实现)
run_test "目录压缩 (内部实现)" \
    "$CAZIP compress -f zip test_dir.zip test_directory" \
    "[ -f test_dir.zip ] && [ -s test_dir.zip ]"

# 测试用例4: 目录解压 (内部实现)
mkdir -p extract_dir
run_test "目录解压 (内部实现)" \
    "$CAZIP extract -f zip extract_dir test_dir.zip" \
    "[ -d extract_dir ] && [ -f extract_dir/file1.txt ]"

# 测试用例5: 单文件压缩 (外部命令)
run_test "单文件压缩 (外部命令)" \
    "$CAZIP compress -e -f zip single_file_ext.zip single_file.txt" \
    "[ -f single_file_ext.zip ] && [ -s single_file_ext.zip ]"

# 测试用例6: 单文件解压 (外部命令)
mkdir -p extract_single_ext
run_test "单文件解压 (外部命令)" \
    "$CAZIP extract -e -f zip extract_single_ext single_file_ext.zip" \
    "[ -f extract_single_ext/single_file.txt ] && diff single_file.txt extract_single_ext/single_file.txt"

# 测试用例7: 目录压缩 (外部命令)
run_test "目录压缩 (外部命令)" \
    "$CAZIP compress -e -f zip test_dir_ext.zip test_directory" \
    "[ -f test_dir_ext.zip ] && [ -s test_dir_ext.zip ]"

# 测试用例8: 目录解压 (外部命令)
mkdir -p extract_dir_ext
run_test "目录解压 (外部命令)" \
    "$CAZIP extract -e -f zip extract_dir_ext test_dir_ext.zip" \
    "[ -d extract_dir_ext/test_directory ] && [ -f extract_dir_ext/test_directory/file1.txt ]"

# 测试用例9: 带密码的压缩
run_test "带密码的压缩" \
    "$CAZIP compress -f zip -p test123 encrypted.zip single_file.txt" \
    "[ -f encrypted.zip ] && [ -s encrypted.zip ]"

# 测试用例10: 带密码的解压
mkdir -p extract_encrypted
run_test "带密码的解压" \
    "$CAZIP extract -f zip -p test123 extract_encrypted encrypted.zip" \
    "[ -f extract_encrypted/single_file.txt ] && diff single_file.txt extract_encrypted/single_file.txt"

# 测试用例11: 压缩大文件
run_test "大文件压缩" \
    "$CAZIP compress -f zip large_file.zip large_file.bin" \
    "[ -f large_file.zip ] && [ -s large_file.zip ]"

# 测试用例12: 解压大文件
mkdir -p extract_large
run_test "大文件解压" \
    "$CAZIP extract -f zip extract_large large_file.zip" \
    "[ -f extract_large/large_file.bin ] && diff large_file.bin extract_large/large_file.bin"

# 测试用例13: 压缩多个文件
run_test "多文件压缩" \
    "$CAZIP compress -f zip multi_files.zip single_file.txt test_directory/file1.txt" \
    "[ -f multi_files.zip ] && [ -s multi_files.zip ]"

# 测试用例14: 解压多文件压缩包
mkdir -p extract_multi
run_test "多文件解压" \
    "$CAZIP extract -f zip extract_multi multi_files.zip" \
    "[ -f extract_multi/single_file.txt ] && [ -f extract_multi/file1.txt ]"

# 测试用例15: 压缩中使用不同压缩方法 (deflated)
run_test "使用deflated方法压缩" \
    "$CAZIP compress -f zip -m deflated method_deflated.zip single_file.txt" \
    "[ -f method_deflated.zip ] && [ -s method_deflated.zip ]"

# 测试用例16: 压缩中使用不同压缩方法 (bzip2)
run_test "使用bzip2方法压缩" \
    "$CAZIP compress -f zip -m bzip2 method_bzip2.zip single_file.txt" \
    "[ -f method_bzip2.zip ] && [ -s method_bzip2.zip ]"

# 测试用例17: 压缩中使用不同压缩方法 (zstd)
run_test "使用zstd方法压缩" \
    "$CAZIP compress -f zip -m zstd method_zstd.zip single_file.txt" \
    "[ -f method_zstd.zip ] && [ -s method_zstd.zip ]"

## 测试用例18: 保留文件权限
#mkdir -p extract_perms
#run_test "保留文件权限" \
#    "$CAZIP -f zip perms.zip executable.sh && $CAZIP -u -f zip extract_perms perms.zip" \
#    "[ -f extract_perms/executable.sh ] && [ -x extract_perms/executable.sh ]"

# 测试用例19: 分卷压缩 (仅在外部命令模式下测试)
#echo -e "${YELLOW}[INFO]${NC} 创建足够大的文件以触发分卷..."
#dd if=/dev/urandom of=large_for_split.bin bs=1M count=200 2>/dev/null
#
#run_test "分卷压缩 (外部命令)" \
#    "$CAZIP -e -f zip -v 1 vol_split.zip test_directory large_for_split.bin" \
#    "[ -f vol_split.zip.001 ] || [ -f vol_split.zip.0001 ]"
#
## 分卷解压测试
#if [ -f vol_split.zip.001 ] || [ -f vol_split.zip.0001 ]; then
#    mkdir -p extract_vol
#    vol_file=$(ls vol_split.zip.* | head -1)
#    run_test "分卷解压 (外部命令)" \
#        "$CAZIP -e -u -f zip extract_vol $vol_file" \
#        "[ -d extract_vol/test_directory ]"
#else
#    echo -e "${YELLOW}[SKIPPED]${NC} 分卷解压测试 - 分卷文件未找到"
#fi

# 测试用例20: 文件列表功能
if $CAZIP -l -f zip single_file.zip >/dev/null 2>&1; then
    run_test "文件列表功能" \
        "$CAZIP -l -f zip single_file.zip > list_output.txt" \
        "[ -s list_output.txt ]"
else
    echo -e "${YELLOW}[SKIPPED]${NC} 文件列表功能 - 不支持"
fi

# 测试用例20: 压缩等级1
run_test "压缩等级1" \
    "$CAZIP compress -f zip --level 1 level1.zip single_file.txt" \
    "[ -f level1.zip ] && [ -s level1.zip ]"

mkdir -p extract_level1
run_test "解压等级1压缩包" \
    "$CAZIP extract -f zip extract_level1 level1.zip" \
    "[ -f extract_level1/single_file.txt ] && diff single_file.txt extract_level1/single_file.txt"

# 测试用例21: 压缩等级5
run_test "压缩等级5" \
    "$CAZIP compress -f zip --level 5 level5.zip single_file.txt" \
    "[ -f level5.zip ] && [ -s level5.zip ]"

mkdir -p extract_level5
run_test "解压等级5压缩包" \
    "$CAZIP extract -f zip extract_level5 level5.zip" \
    "[ -f extract_level5/single_file.txt ] && diff single_file.txt extract_level5/single_file.txt"

# 测试用例22: 压缩等级9
run_test "压缩等级9" \
    "$CAZIP compress -f zip --level 9 level9.zip single_file.txt" \
    "[ -f level9.zip ] && [ -s level9.zip ]"

mkdir -p extract_level9
run_test "解压等级9压缩包" \
    "$CAZIP extract -f zip extract_level9 level9.zip" \
    "[ -f extract_level9/single_file.txt ] && diff single_file.txt extract_level9/single_file.txt"

add_partial_extraction_tests() {
    local format=$1
    local format_ext=$2
    local cazip=$3

    # 为部分提取测试创建特殊的测试数据
    echo -e "\n${YELLOW}[INFO]${NC} 创建部分提取测试的数据..."

    # 创建有特定目录结构的测试目录
    mkdir -p test_structure/dir1
    mkdir -p test_structure/dir2/subdir

    # 创建不同格式的测试文件
    echo "这是文本文件1" > test_structure/file1.txt
    echo "这是文本文件2" > test_structure/dir1/file2.txt
    echo "这是文本文件3" > test_structure/dir2/file3.txt
    echo "这是子目录中的文件" > test_structure/dir2/subdir/file4.txt

    # 创建其他类型的文件
    dd if=/dev/urandom of=test_structure/binary1.bin bs=1k count=5 2>/dev/null
    dd if=/dev/urandom of=test_structure/dir1/binary2.bin bs=1k count=5 2>/dev/null

    # 压缩测试目录
    case $format in
        "zip")
            $cazip compress -e test_structure.$format_ext test_structure
            ;;
        "7z")
            $cazip compress -e -f $format test_structure.$format_ext test_structure
            ;;
        "xz"|"gz")
            # 对于xz和gz，需要先创建tar文件
            tar -cf test_structure.tar test_structure
            $cazip compress -f $format test_structure.tar.$format_ext test_structure.tar
            ;;
    esac

    echo -e "${GREEN}[SUCCESS]${NC} 部分提取测试数据创建完成"

    # 测试用例：提取单个文件
    mkdir -p extract_single_file
    run_test "提取单个文件" \
        "$cazip extract -e --files test_structure/file1.txt -f $format extract_single_file test_structure.$format_ext" \
        "[ -f extract_single_file/test_structure/file1.txt ] && [ ! -f extract_single_file/test_structure/dir1/file2.txt ]"

    # 测试用例：提取特定目录
    mkdir -p extract_specific_dir
    run_test "提取特定目录" \
        "$cazip extract -e --files 'test_structure/dir1' -f $format extract_specific_dir test_structure.$format_ext" \
        "[ -d extract_specific_dir/test_structure/dir1 ] && [ -f extract_specific_dir/test_structure/dir1/file2.txt ] && [ ! -f extract_specific_dir/test_structure/file1.txt ]"

    # 测试用例：提取多个文件
    mkdir -p extract_multiple_files
    run_test "提取多个文件" \
        "$cazip extract -e -f $format extract_multiple_files test_structure.$format_ext --files test_structure/file1.txt,test_structure/dir2/file3.txt" \
        "[ -f extract_multiple_files/test_structure/file1.txt ] && [ -f extract_multiple_files/test_structure/dir2/file3.txt ] && [ ! -f extract_multiple_files/test_structure/dir1/file2.txt ]"

    # 测试用例：提取使用通配符（仅适用于支持通配符的格式）
    if [ "$format" = "zip" ] || [ "$format" = "7z" ]; then
        mkdir -p extract_wildcard
        run_test "使用通配符提取文件" \
            "$cazip extract -e --files 'test_structure/*.txt' -f $format extract_wildcard test_structure.$format_ext" \
            "[ -f extract_wildcard/test_structure/file1.txt ] && [ ! -f extract_wildcard/test_structure/binary1.bin ]"
    fi

    # 测试用例：提取子目录中的文件
    mkdir -p extract_nested
    run_test "提取嵌套子目录中的文件" \
        "$cazip extract -e --files 'test_structure/dir2/subdir/file4.txt' -f $format extract_nested test_structure.$format_ext" \
        "[ -f extract_nested/test_structure/dir2/subdir/file4.txt ] && [ ! -f extract_nested/test_structure/dir2/file3.txt ]"

    # 测试用例：尝试提取不存在的文件（应该给出错误）
    run_test "尝试提取不存在的文件" \
        "$cazip extract -e --files 'nonexistent_file.txt' -f $format extract_error test_structure.$format_ext 2>&1 | grep -q 'Error\|错误\|not found'" \
        "[ $? -eq 0 ]"
}

add_partial_extraction_tests "zip" "zip" "$CAZIP"
# ====== 测试用例结束 ======

# 打印测试结果摘要
echo -e "\n${YELLOW}======= 测试摘要 =======${NC}"
echo -e "总共测试: $TESTS_TOTAL"
echo -e "通过测试: $TESTS_PASSED"
echo -e "失败测试: $((TESTS_TOTAL - TESTS_PASSED))"

if [ $TESTS_PASSED -eq $TESTS_TOTAL ]; then
    echo -e "${GREEN}所有测试通过!${NC}"
    exit_code=0
else
    echo -e "${RED}有测试失败${NC}"
    exit_code=1
fi

# 询问是否清理测试目录
read -p "是否清理测试目录? (y/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    cd ..
    rm -rf "$TEST_DIR"
    echo -e "${GREEN}已清理测试目录${NC}"
fi

exit $exit_code