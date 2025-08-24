cmake_minimum_required(VERSION 3.22.1)
project(aubo_module)

# Set C++ standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Android specific settings
set(CMAKE_SYSTEM_NAME Android)
set(CMAKE_ANDROID_API 29)
set(CMAKE_ANDROID_ARCH_ABI arm64-v8a)

# Find required libraries
find_library(log-lib log)
find_library(dl-lib dl)

# Include directories
include_directories(${CMAKE_SOURCE_DIR})
include_directories(${CMAKE_SOURCE_DIR}/../..)

# Create shared library for ZygiskNext module
add_library(aubo_module SHARED
    aubo_module.cpp
)

# Link libraries
target_link_libraries(aubo_module
    ${log-lib}
    ${dl-lib}
)

# Set output name and directory
set_target_properties(aubo_module PROPERTIES
    OUTPUT_NAME "aubo_module"
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}"
)

# Strip symbols in release builds (disabled to preserve ZygiskNext exports)
if(CMAKE_BUILD_TYPE STREQUAL "Release")
    # Keep symbols for ZygiskNext module loading
    # set_target_properties(aubo_module PROPERTIES LINK_FLAGS "-s")
endif()

# Compiler flags
target_compile_options(aubo_module PRIVATE
    -Wall
    -Wextra
    -fvisibility=default
    -ffunction-sections
    -fdata-sections
)

# Linker flags
target_link_options(aubo_module PRIVATE
    -Wl,--export-dynamic
    -Wl,--exclude-libs,ALL
)