#!/bin/bash

# 测试 cazip 程序的 GZ 压缩/解压功能的脚本
# 使用方法: ./test_gz.sh [cazip路径]

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
TEST_DIR="cazip_test_$(date +%s)"
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

echo -e "${GREEN}[SUCCESS]${NC} 测试数据准备完成"

# ====== 测试用例开始 ======

# 测试用例1: 单文件压缩 (内部实现)
run_test "单文件压缩 (内部实现)" \
    "$CAZIP -f gz single_file.txt.gz single_file.txt" \
    "[ -f single_file.txt.gz ] && [ -s single_file.txt.gz ]"

# 测试用例2: 单文件解压 (内部实现)
mkdir -p extract_single
run_test "单文件解压 (内部实现)" \
    "$CAZIP -u -f gz extract_single/single_file.txt single_file.txt.gz" \
    "[ -f extract_single/single_file.txt ] && diff single_file.txt extract_single/single_file.txt"

# 测试用例3: 目录压缩 (内部实现)
run_test "目录压缩 (内部实现)" \
    "$CAZIP -f gz test_dir.tar.gz test_directory" \
    "[ -f test_dir.tar.gz ] && [ -s test_dir.tar.gz ]"

# 测试用例4: 目录解压 (内部实现)
mkdir -p extract_dir
run_test "目录解压 (内部实现)" \
    "$CAZIP -u -f gz extract_dir test_dir.tar.gz" \
    "[ -d extract_dir/test_directory ] && [ -f extract_dir/test_directory/file1.txt ]"

# 测试用例5: 单文件压缩 (外部命令)
run_test "单文件压缩 (外部命令)" \
    "$CAZIP -e -f gz single_file_ext.gz single_file.txt" \
    "[ -f single_file_ext.gz ] && [ -s single_file_ext.gz ]"

# 测试用例6: 单文件解压 (外部命令)
mkdir -p extract_single_ext
run_test "单文件解压 (外部命令)" \
    "$CAZIP -e -u -f gz extract_single_ext/single_file.txt single_file_ext.gz" \
    "[ -f extract_single_ext/single_file.txt ] && diff single_file.txt extract_single_ext/single_file.txt"

# 测试用例7: 目录压缩 (外部命令)
run_test "目录压缩 (外部命令)" \
    "$CAZIP -e -f gz test_dir_ext.tar.gz test_directory" \
    "[ -f test_dir_ext.tar.gz ] && [ -s test_dir_ext.tar.gz ]"

# 测试用例8: 目录解压 (外部命令)
mkdir -p extract_dir_ext
run_test "目录解压 (外部命令)" \
    "$CAZIP -e -u -f gz extract_dir_ext test_dir_ext.tar.gz" \
    "[ -d extract_dir_ext/test_directory ] && [ -f extract_dir_ext/test_directory/file1.txt ]"

# 测试用例9: 压缩大文件
run_test "大文件压缩" \
    "$CAZIP -e -f gz large_file.bin.gz large_file.bin" \
    "[ -f large_file.bin.gz ] && [ -s large_file.bin.gz ]"

# 测试用例10: 解压大文件
mkdir -p extract_large
run_test "大文件解压" \
    "$CAZIP -e -u -f gz extract_large/large_file.bin large_file.bin.gz" \
    "[ -f extract_large/large_file.bin ] && diff large_file.bin extract_large/large_file.bin"

# 测试用例11: 压缩多个文件
run_test "多文件压缩" \
    "$CAZIP -e -f gz multi_files.tar.gz single_file.txt test_directory/file1.txt" \
    "[ -f multi_files.tar.gz ] && [ -s multi_files.tar.gz ]"

# 测试用例12: 解压多文件压缩包
mkdir -p extract_multi
run_test "多文件解压" \
    "$CAZIP -e -u -f gz extract_multi multi_files.tar.gz" \
    "[ -f extract_multi/single_file.txt ] && [ -f extract_multi/test_directory/file1.txt ]"

# 测试用例13: 错误处理 - 不存在的文件
run_test "错误处理 - 不存在的文件" \
    "$CAZIP -f gz nonexist.gz nonexistent_file.txt 2>&1 | grep -q 'Error'" \
    "[ $? -eq 0 ]"

# 测试用例14: 开启调试模式
run_test "调试模式" \
    "$CAZIP -d -f gz debug.gz single_file.txt 2>&1 | grep -q 'debug'" \
    "[ -f debug.gz ]"

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