#!/bin/bash

# 测试 cazip 程序的 XZ 压缩/解压功能的脚本
# 使用方法: ./test_xz.sh [cazip路径]

# 设置颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;36m'
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
TEST_DIR="cazip_xz_test_$(date +%s)"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}XZ 格式压缩/解压功能测试${NC}"
echo -e "${BLUE}======================================${NC}"

echo -e "${YELLOW}[INFO]${NC} 测试目录: $PWD"
echo -e "${YELLOW}[INFO]${NC} 使用的 cazip: $CAZIP"

# 检查 xz 命令是否可用（外部命令测试需要）
if ! command -v xz &> /dev/null; then
    echo -e "${YELLOW}[WARNING]${NC} xz 命令不可用，外部命令测试可能会失败"
fi

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

# 创建高度可压缩的文件
dd if=/dev/zero of=zeros.bin bs=100k count=1 2>/dev/null

# 创建原始tar文件以测试tar.xz
tar -cf test_content.tar test_directory

echo -e "${GREEN}[SUCCESS]${NC} 测试数据准备完成"

# ====== 测试用例开始 ======

# 测试用例1: 单文件压缩 (内部实现)
run_test "单文件压缩 (内部实现)" \
    "$CAZIP compress -f xz single_file.xz single_file.txt" \
    "[ -f single_file.xz ] && [ -s single_file.xz ]"

# 测试用例2: 单文件解压 (内部实现)
mkdir -p extract_single
run_test "单文件解压 (内部实现)" \
    "$CAZIP extract -f xz extract_single single_file.xz" \
    "[ -f extract_single/single_file.txt ] && diff single_file.txt extract_single/single_file.txt"

# 测试用例3: 单文件压缩 (外部命令)
run_test "单文件压缩 (外部命令)" \
    "$CAZIP compress -e -f xz single_file_ext.xz single_file.txt" \
    "[ -f single_file_ext.xz ] && [ -s single_file_ext.xz ]"

# 测试用例4: 单文件解压 (外部命令)
mkdir -p extract_single_ext
run_test "单文件解压 (外部命令)" \
    "$CAZIP extract -e -f xz extract_single_ext single_file_ext.xz" \
    "[ -f extract_single_ext/single_file.txt ] && diff single_file.txt extract_single_ext/single_file.txt"

# 测试用例5: tar创建并压缩 (外部命令)
run_test "目录压缩为tar.xz (外部命令)" \
    "$CAZIP compress -e -f xz test_dir.tar.xz test_directory" \
    "[ -f test_dir.tar.xz ] && [ -s test_dir.tar.xz ]"

# 测试用例6: tar.xz解压 (外部命令)
mkdir -p extract_dir
run_test "tar.xz目录解压 (外部命令)" \
    "$CAZIP extract -e -f xz extract_dir test_dir.tar.xz" \
    "[ -d extract_dir/test_directory ] && [ -f extract_dir/test_directory/file1.txt ]"

# 测试用例7: 压缩已存在的tar文件
run_test "压缩已存在的tar文件" \
    "$CAZIP compress -f xz test_content.tar.xz test_content.tar" \
    "[ -f test_content.tar.xz ] && [ -s test_content.tar.xz ]"

# 测试用例8: 解压tar.xz文件到tar
mkdir -p extract_tar
run_test "解压tar.xz到tar文件" \
    "$CAZIP extract -e -f xz extract_tar test_content.tar.xz" \
    "[ -f extract_tar/test_content.tar ] && tar -tf extract_tar/test_content.tar > /dev/null"

# 测试用例9: 压缩大文件
run_test "大文件压缩" \
    "$CAZIP compress -f xz large_file.xz large_file.bin" \
    "[ -f large_file.xz ] && [ -s large_file.xz ]"

# 测试用例10: 解压大文件
mkdir -p extract_large
run_test "大文件解压" \
    "$CAZIP extract -e -f xz extract_large large_file.xz" \
    "[ -f extract_large/large_file.bin ] && diff large_file.bin extract_large/large_file.bin"

# 测试用例11: 多线程压缩 (如果支持)
# XZ 编解码器应该支持多线程压缩
run_test "多线程压缩" \
    "$CAZIP compress -f xz -d mt_compress.xz large_file.bin" \
    "[ -f mt_compress.xz ] && [ -s mt_compress.xz ]"

# 测试用例12: 高压缩率测试 (全零文件)
run_test "高压缩率文件测试" \
    "$CAZIP compress -f xz zeros.xz zeros.bin" \
    "[ -f zeros.xz ] && [ \$(stat -c%s zeros.xz) -lt \$(stat -c%s zeros.bin) ]"

# 测试用例13: 解压到标准输出 (如果支持)
# 很多xz实现支持解压到标准输出
if $CAZIP -f xz - single_file.xz > /dev/null 2>&1; then
    run_test "解压到标准输出" \
        "$CAZIP -f xz - single_file.xz > stdout_output.txt" \
        "[ -s stdout_output.txt ] && diff single_file.txt stdout_output.txt"
else
    echo -e "${YELLOW}[SKIPPED]${NC} 解压到标准输出测试 - 不支持"
fi

# 测试用例14: 压缩级别测试 (如果支持)
# 尝试不同的压缩级别
for level in 1 3 6 9; do
    if $CAZIP -f xz -m "level=$level" level_test.xz single_file.txt > /dev/null 2>&1; then
        run_test "压缩级别 $level 测试" \
            "$CAZIP -f xz -m \"level=$level\" level_${level}.xz single_file.txt" \
            "[ -f level_${level}.xz ]"
    else
        echo -e "${YELLOW}[SKIPPED]${NC} 压缩级别 $level 测试 - 不支持"
    fi
done

# 测试用例15: 使用.txz扩展名 (tar+xz的简写)
run_test "使用.txz扩展名压缩" \
    "$CAZIP compress -e -f xz test_dir.txz test_directory" \
    "[ -f test_dir.txz ] && [ -s test_dir.txz ]"

mkdir -p extract_txz
run_test "解压.txz文件" \
    "$CAZIP extract -e -f xz extract_txz test_dir.txz" \
    "[ -d extract_txz/test_directory ]"

# 测试用例16: 文件列表功能 (如果支持)
if $CAZIP -l -f xz test_dir.tar.xz >/dev/null 2>&1; then
    run_test "文件列表功能" \
        "$CAZIP -l -f xz test_dir.tar.xz > list_output.txt" \
        "[ -s list_output.txt ]"
else
    echo -e "${YELLOW}[SKIPPED]${NC} 文件列表功能 - 不支持"
fi

# 测试用例17: 保留原始文件(如果支持)
if [ -f single_file.txt ]; then
    cp single_file.txt preserve_test.txt
    run_test "压缩保留原始文件" \
        "$CAZIP compress -f xz preserve_test.xz preserve_test.txt" \
        "[ -f preserve_test.xz ] && [ -f preserve_test.txt ]"
fi

# 测试用例18: 多文件压缩 (预期会创建tar.xz)
run_test "多文件压缩" \
    "$CAZIP compress -f xz multi_files.xz single_file.txt large_file.bin" \
    "[ -f multi_files.xz ] && [ -s multi_files.xz ]"

mkdir -p extract_multi
run_test "多文件解压" \
    "$CAZIP extract -e -f xz extract_multi multi_files.xz" \
    "[ -f extract_multi/single_file.txt ] && [ -f extract_multi/large_file.bin ]"

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
            $cazip compress -f $format -e test_structure.$format_ext test_structure
            ;;
        "7z")
            $cazip compress -e -f $format test_structure.$format_ext test_structure
            ;;
        "xz"|"gz")
            # 对于xz和gz，需要先创建tar文件
            $cazip compress-e -f $format test_structure.tar.$format_ext test_structure
            ;;
    esac

    echo -e "${GREEN}[SUCCESS]${NC} 部分提取测试数据创建完成"

    # 测试用例：提取单个文件
    mkdir -p extract_single_file
    run_test "提取单个文件" \
        "$cazip extract -e --files test_structure/file1.txt -f $format extract_single_file test_structure.tar.$format_ext" \
        "[ -f extract_single_file/test_structure/file1.txt ] && [ ! -f extract_single_file/test_structure/dir1/file2.txt ]"

    # 测试用例：提取特定目录
    mkdir -p extract_specific_dir
    run_test "提取特定目录" \
        "$cazip extract -e --files 'test_structure/dir1' -f $format extract_specific_dir test_structure.tar.$format_ext" \
        "[ -d extract_specific_dir/test_structure/dir1 ] && [ -f extract_specific_dir/test_structure/dir1/file2.txt ] && [ ! -f extract_specific_dir/test_structure/file1.txt ]"

    # 测试用例：提取多个文件
    mkdir -p extract_multiple_files
    run_test "提取多个文件" \
        "$cazip extract -e -f $format extract_multiple_files test_structure.tar.$format_ext --files test_structure/file1.txt,test_structure/dir2/file3.txt" \
        "[ -f extract_multiple_files/test_structure/file1.txt ] && [ -f extract_multiple_files/test_structure/dir2/file3.txt ] && [ ! -f extract_multiple_files/test_structure/dir1/file2.txt ]"

    # 测试用例：提取使用通配符（仅适用于支持通配符的格式）
    if [ "$format" = "zip" ] || [ "$format" = "7z" ]; then
        mkdir -p extract_wildcard
        run_test "使用通配符提取文件" \
            "$cazip extract -e --files 'test_structure/*.txt' -f $format extract_wildcard test_structure.tar.$format_ext" \
            "[ -f extract_wildcard/test_structure/file1.txt ] && [ ! -f extract_wildcard/test_structure/binary1.bin ]"
    fi

    # 测试用例：提取子目录中的文件
    mkdir -p extract_nested
    run_test "提取嵌套子目录中的文件" \
        "$cazip extract -e --files 'test_structure/dir2/subdir/file4.txt' -f $format extract_nested test_structure.tar.$format_ext" \
        "[ -f extract_nested/test_structure/dir2/subdir/file4.txt ] && [ ! -f extract_nested/test_structure/dir2/file3.txt ]"

    # 测试用例：尝试提取不存在的文件（应该给出错误）
    run_test "尝试提取不存在的文件" \
        "$cazip extract -e --files 'nonexistent_file.txt' -f $format extract_error test_structure.tar.$format_ext 2>&1 | grep -q 'Error\|错误\|not found'" \
        "[ $? -eq 0 ]"
}

add_partial_extraction_tests "xz" "xz" "$CAZIP"

# ====== 压缩等级测试 ======

# native模式
run_test "xz压缩等级1（native）" \
    "$CAZIP compress -f xz --level 1 level1.xz single_file.txt" \
    "[ -f level1.xz ] && [ -s level1.xz ]"

mkdir -p extract_level1_xz
run_test "xz解压等级1压缩包（native）" \
    "$CAZIP extract -f xz extract_level1_xz level1.xz" \
    "[ -f extract_level1_xz/single_file.txt ] && diff single_file.txt extract_level1_xz/single_file.txt"

run_test "xz压缩等级5（native）" \
    "$CAZIP compress -f xz --level 5 level5.xz single_file.txt" \
    "[ -f level5.xz ] && [ -s level5.xz ]"

mkdir -p extract_level5_xz
run_test "xz解压等级5压缩包（native）" \
    "$CAZIP extract -f xz extract_level5_xz level5.xz" \
    "[ -f extract_level5_xz/single_file.txt ] && diff single_file.txt extract_level5_xz/single_file.txt"

run_test "xz压缩等级9（native）" \
    "$CAZIP compress -f xz --level 9 level9.xz single_file.txt" \
    "[ -f level9.xz ] && [ -s level9.xz ]"

mkdir -p extract_level9_xz
run_test "xz解压等级9压缩包（native）" \
    "$CAZIP extract -f xz extract_level9_xz level9.xz" \
    "[ -f extract_level9_xz/single_file.txt ] && diff single_file.txt extract_level9_xz/single_file.txt"

# 外部命令模式
run_test "xz压缩等级1（外部命令）" \
    "$CAZIP compress -e -f xz --level 1 level1_ext.xz single_file.txt" \
    "[ -f level1_ext.xz ] && [ -s level1_ext.xz ]"

mkdir -p extract_level1_xz_ext
run_test "xz解压等级1压缩包（外部命令）" \
    "$CAZIP extract -e -f xz extract_level1_xz_ext level1_ext.xz" \
    "[ -f extract_level1_xz_ext/single_file.txt ] && diff single_file.txt extract_level1_xz_ext/single_file.txt"

run_test "xz压缩等级5（外部命令）" \
    "$CAZIP compress -e -f xz --level 5 level5_ext.xz single_file.txt" \
    "[ -f level5_ext.xz ] && [ -s level5_ext.xz ]"

mkdir -p extract_level5_xz_ext
run_test "xz解压等级5压缩包（外部命令）" \
    "$CAZIP extract -e -f xz extract_level5_xz_ext level5_ext.xz" \
    "[ -f extract_level5_xz_ext/single_file.txt ] && diff single_file.txt extract_level5_xz_ext/single_file.txt"

run_test "xz压缩等级9（外部命令）" \
    "$CAZIP compress -e -f xz --level 9 level9_ext.xz single_file.txt" \
    "[ -f level9_ext.xz ] && [ -s level9_ext.xz ]"

mkdir -p extract_level9_xz_ext
run_test "xz解压等级9压缩包（外部命令）" \
    "$CAZIP extract -e -f xz extract_level9_xz_ext level9_ext.xz" \
    "[ -f extract_level9_xz_ext/single_file.txt ] && diff single_file.txt extract_level9_xz_ext/single_file.txt"

# ====== 测试用例结束 ======

# 打印测试结果摘要
echo -e "\n${BLUE}======================================${NC}"
echo -e "${YELLOW}======= 测试摘要 =======${NC}"
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