#!/bin/bash
# Publishing script for dataweave-py

set -e  # Exit on error

echo "🚀 Publishing dataweave-py to PyPI"
echo "=================================="
echo ""

# Check if we're in the right directory
if [ ! -f "pyproject.toml" ]; then
    echo "❌ Error: pyproject.toml not found. Run this script from the project root."
    exit 1
fi

# Parse command line arguments
TARGET="pypi"
SKIP_TESTS=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --test)
            TARGET="testpypi"
            shift
            ;;
        --skip-tests)
            SKIP_TESTS=true
            shift
            ;;
        *)
            echo "❌ Unknown option: $1"
            echo "Usage: $0 [--test] [--skip-tests]"
            echo "  --test       Upload to Test PyPI instead of PyPI"
            echo "  --skip-tests Skip running tests"
            exit 1
            ;;
    esac
done

# Step 1: Run tests
if [ "$SKIP_TESTS" = false ]; then
    echo "📋 Step 1: Running tests..."
    if command -v pytest &> /dev/null; then
        pytest tests/ || {
            echo "❌ Tests failed. Fix them before publishing."
            exit 1
        }
        echo "✅ Tests passed!"
    else
        echo "⚠️  pytest not found. Skipping tests."
    fi
else
    echo "⏭️  Skipping tests (--skip-tests flag used)"
fi
echo ""

# Step 2: Clean previous builds
echo "🧹 Step 2: Cleaning previous builds..."
rm -rf build/ dist/ *.egg-info/
echo "✅ Cleaned!"
echo ""

# Step 3: Build the package
echo "🔨 Step 3: Building package..."
if ! command -v python &> /dev/null; then
    echo "❌ Python not found. Please install Python 3.10+."
    exit 1
fi

python -m build || {
    echo "❌ Build failed. Install build tools: pip install build"
    exit 1
}
echo "✅ Package built!"
echo ""

# Step 4: Check the package
echo "🔍 Step 4: Checking package..."
if ! command -v twine &> /dev/null; then
    echo "❌ twine not found. Install it: pip install twine"
    exit 1
fi

twine check dist/* || {
    echo "❌ Package check failed."
    exit 1
}
echo "✅ Package looks good!"
echo ""

# Step 5: Upload
if [ "$TARGET" = "testpypi" ]; then
    echo "📤 Step 5: Uploading to Test PyPI..."
    echo "⚠️  You'll need your Test PyPI API token"
    echo ""
    twine upload --repository testpypi dist/* || {
        echo "❌ Upload to Test PyPI failed."
        exit 1
    }
    echo ""
    echo "✅ Successfully uploaded to Test PyPI!"
    echo ""
    echo "📦 Test your package with:"
    echo "   pip install --index-url https://test.pypi.org/simple/ dataweave-py"
    echo ""
    echo "🔗 View at: https://test.pypi.org/project/dataweave-py/"
else
    echo "📤 Step 5: Uploading to PyPI..."
    echo "⚠️  You'll need your PyPI API token"
    echo ""
    read -p "Are you sure you want to publish to PyPI? (yes/no): " -r
    echo
    if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
        echo "❌ Upload cancelled."
        exit 1
    fi
    
    twine upload dist/* || {
        echo "❌ Upload to PyPI failed."
        exit 1
    }
    echo ""
    echo "✅ Successfully published to PyPI!"
    echo ""
    echo "📦 Install with:"
    echo "   pip install dataweave-py"
    echo ""
    echo "🔗 View at: https://pypi.org/project/dataweave-py/"
fi

echo ""
echo "🎉 All done!"



