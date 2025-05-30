#!/bin/bash

# 测试 cazip 程序的 7Z 压缩/解压功能的脚本
# 使用方法: ./test_7z.sh [cazip路径]

# 设置颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
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
TEST_DIR="cazip_7z_test_$(date +%s)"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}7Z 格式压缩/解压功能测试${NC}"
echo -e "${BLUE}======================================${NC}"

echo -e "${YELLOW}[INFO]${NC} 测试目录: $PWD"
echo -e "${YELLOW}[INFO]${NC} 使用的 cazip: $CAZIP"

# 检查 7z 命令是否可用（外部命令测试需要）
if ! command -v 7z &> /dev/null; then
    echo -e "${YELLOW}[WARNING]${NC} 7z 命令不可用，外部命令测试可能会失败"
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

# 创建一些特殊内容的文件
echo "包含中文字符的文件" > chinese_text.txt
echo -e "\xff\xfe\x31\x00\x32\x00\x33\x00" > utf16_file.txt
dd if=/dev/zero of=zeros.bin bs=100k count=1 2>/dev/null

# 创建具有特殊名称的文件
echo "特殊文件名内容" > "special filename with spaces.txt"
echo "文件名包含中文" > "中文文件名.txt"

echo -e "${GREEN}[SUCCESS]${NC} 测试数据准备完成"

# ====== 测试用例开始 ======

# 测试用例1: 单文件压缩 (内部实现)
# 注意：如果内部实现不完整，这个测试可能会失败
#run_test "单文件压缩 (内部实现)" \
#    "$CAZIP -f 7z single_file.7z single_file.txt" \
#    "[ -f single_file.7z ] && [ -s single_file.7z ]"
#
## 测试用例2: 单文件解压 (内部实现)
## 注意：如果内部实现不完整，这个测试可能会失败
#mkdir -p extract_single
#run_test "单文件解压 (内部实现)" \
#    "$CAZIP -u -f 7z extract_single single_file.7z" \
#    "[ -f extract_single/single_file.txt ]"

# 测试用例3: 单文件压缩 (外部命令)
run_test "单文件压缩 (外部命令)" \
    "$CAZIP compress -e -f 7z single_file_ext.7z single_file.txt" \
    "[ -f single_file_ext.7z ] && [ -s single_file_ext.7z ]"

# 测试用例4: 单文件解压 (外部命令)
mkdir -p extract_single_ext
run_test "单文件解压 (外部命令)" \
    "$CAZIP extract -e -f 7z extract_single_ext single_file_ext.7z" \
    "[ -f extract_single_ext/single_file.txt ] && diff single_file.txt extract_single_ext/single_file.txt"

# 测试用例5: 目录压缩 (外部命令)
run_test "目录压缩 (外部命令)" \
    "$CAZIP compress -e -f 7z test_dir.7z test_directory" \
    "[ -f test_dir.7z ] && [ -s test_dir.7z ]"

# 测试用例6: 目录解压 (外部命令)
mkdir -p extract_dir
run_test "目录解压 (外部命令)" \
    "$CAZIP extract -e -f 7z extract_dir test_dir.7z" \
    "[ -d extract_dir/test_directory ] && [ -f extract_dir/test_directory/file1.txt ]"

# 测试用例7: 带密码的压缩
run_test "带密码的压缩" \
    "$CAZIP compress -e -f 7z -p test123 encrypted.7z single_file.txt" \
    "[ -f encrypted.7z ] && [ -s encrypted.7z ]"

# 测试用例8: 带密码的解压
mkdir -p extract_encrypted
run_test "带密码的解压" \
    "$CAZIP extract -e -f 7z -p test123 extract_encrypted encrypted.7z" \
    "[ -f extract_encrypted/single_file.txt ] && diff single_file.txt extract_encrypted/single_file.txt"

# 测试用例9: 压缩大文件
run_test "大文件压缩" \
    "$CAZIP compress -e -f 7z large_file.7z large_file.bin" \
    "[ -f large_file.7z ] && [ -s large_file.7z ]"

# 测试用例10: 解压大文件
mkdir -p extract_large
run_test "大文件解压" \
    "$CAZIP extract -e -f 7z extract_large large_file.7z" \
    "[ -f extract_large/large_file.bin ] && diff large_file.bin extract_large/large_file.bin"

# 测试用例11: 压缩多个文件
run_test "多文件压缩" \
    "$CAZIP compress -e -f 7z multi_files.7z single_file.txt test_directory/file1.txt" \
    "[ -f multi_files.7z ] && [ -s multi_files.7z ]"

# 测试用例12: 解压多文件压缩包
mkdir -p extract_multi
run_test "多文件解压" \
    "$CAZIP extract -e -f 7z extract_multi multi_files.7z" \
    "[ -f extract_multi/single_file.txt ] && [ -f extract_multi/test_directory/file1.txt ]"

# 测试用例13: 压缩具有特殊名称的文件
run_test "压缩特殊名称文件" \
    "$CAZIP compress -e -f 7z special_names.7z \"special filename with spaces.txt\" \"中文文件名.txt\"" \
    "[ -f special_names.7z ] && [ -s special_names.7z ]"

# 测试用例14: 解压具有特殊名称的文件
mkdir -p extract_special
run_test "解压特殊名称文件" \
    "$CAZIP extract -e -f 7z extract_special special_names.7z" \
    "[ -f \"extract_special/special filename with spaces.txt\" ] && [ -f \"extract_special/中文文件名.txt\" ]"

# 测试用例15: 压缩含中文内容的文件
run_test "压缩中文内容文件" \
    "$CAZIP compress -e -f 7z chinese_content.7z chinese_text.txt" \
    "[ -f chinese_content.7z ] && [ -s chinese_content.7z ]"

# 测试用例16: 解压含中文内容的文件
mkdir -p extract_chinese
run_test "解压中文内容文件" \
    "$CAZIP extract -e -f 7z extract_chinese chinese_content.7z" \
    "[ -f extract_chinese/chinese_text.txt ] && diff chinese_text.txt extract_chinese/chinese_text.txt"

# 测试用例17: 高压缩率测试 (全零文件)
run_test "高压缩率文件测试" \
    "$CAZIP compress -e -f 7z zeros.7z zeros.bin" \
    "[ -f zeros.7z ] && [ \$(stat -c%s zeros.7z) -lt \$(stat -c%s zeros.bin) ]"

# 测试用例18: 分卷压缩 (如果支持)
if $CAZIP -e -f 7z -v 1 vol_test.7z single_file.txt >/dev/null 2>&1; then
    run_test "分卷压缩" \
        "$CAZIP compress -e -f 7z -v 1 vol_split.7z test_directory" \
        "[ -f vol_split.7z.001 ] || [ -f vol_split.7z.0001 ]"

    # 分卷解压测试
    if [ -f vol_split.7z.001 ] || [ -f vol_split.7z.0001 ]; then
        mkdir -p extract_vol
        vol_file=$(ls vol_split.7z.* | head -1)
        run_test "分卷解压" \
            "$CAZIP extract -e -f 7z extract_vol $vol_file" \
            "[ -d extract_vol/test_directory ]"
    fi
else
    echo -e "${YELLOW}[SKIPPED]${NC} 分卷压缩测试 - 不支持"
fi

# 测试用例19: UTF-16 文件处理
run_test "UTF-16 文件压缩" \
    "$CAZIP compress -e -f 7z utf16.7z utf16_file.txt" \
    "[ -f utf16.7z ] && [ -s utf16.7z ]"

mkdir -p extract_utf16
run_test "UTF-16 文件解压" \
    "$CAZIP extract -e -f 7z extract_utf16 utf16.7z" \
    "[ -f extract_utf16/utf16_file.txt ] && cmp utf16_file.txt extract_utf16/utf16_file.txt"

# 测试用例20: 文件列表功能
if $CAZIP -l -f 7z single_file_ext.7z >/dev/null 2>&1; then
    run_test "文件列表功能" \
        "$CAZIP -l -f 7z single_file_ext.7z > list_output.txt" \
        "[ -s list_output.txt ]"
else
    echo -e "${YELLOW}[SKIPPED]${NC} 文件列表功能 - 不支持"
fi

# ====== 压缩等级测试（仅外部命令模式） ======
# native实现暂不支持压缩等级参数

# 测试用例: 7z压缩等级1（外部命令）
run_test "7z压缩等级1（外部命令）" \
    "$CAZIP compress -e -f 7z --level 1 level1.7z single_file.txt" \
    "[ -f level1.7z ] && [ -s level1.7z ]"

mkdir -p extract_level1_7z
run_test "7z解压等级1压缩包（外部命令）" \
    "$CAZIP extract -e -f 7z extract_level1_7z level1.7z" \
    "[ -f extract_level1_7z/single_file.txt ] && diff single_file.txt extract_level1_7z/single_file.txt"

# 测试用例: 7z压缩等级5（外部命令）
run_test "7z压缩等级5（外部命令）" \
    "$CAZIP compress -e -f 7z --level 5 level5.7z single_file.txt" \
    "[ -f level5.7z ] && [ -s level5.7z ]"

mkdir -p extract_level5_7z
run_test "7z解压等级5压缩包（外部命令）" \
    "$CAZIP extract -e -f 7z extract_level5_7z level5.7z" \
    "[ -f extract_level5_7z/single_file.txt ] && diff single_file.txt extract_level5_7z/single_file.txt"

# 测试用例: 7z压缩等级9（外部命令）
run_test "7z压缩等级9（外部命令）" \
    "$CAZIP compress -e -f 7z --level 9 level9.7z single_file.txt" \
    "[ -f level9.7z ] && [ -s level9.7z ]"

mkdir -p extract_level9_7z
run_test "7z解压等级9压缩包（外部命令）" \
    "$CAZIP extract -e -f 7z extract_level9_7z level9.7z" \
    "[ -f extract_level9_7z/single_file.txt ] && diff single_file.txt extract_level9_7z/single_file.txt"

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