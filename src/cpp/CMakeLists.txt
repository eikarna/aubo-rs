cmake_minimum_required(VERSION 3.18.1)
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

# Set output name
set_target_properties(aubo_module PROPERTIES
    OUTPUT_NAME "aubo_module"
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_SOURCE_DIR}/../../lib"
)

# Strip symbols in release builds
if(CMAKE_BUILD_TYPE STREQUAL "Release")
    set_target_properties(aubo_module PROPERTIES
        LINK_FLAGS "-s"
    )
endif()

# Compiler flags
target_compile_options(aubo_module PRIVATE
    -Wall
    -Wextra
    -fvisibility=hidden
    -ffunction-sections
    -fdata-sections
)

# Linker flags
target_link_options(aubo_module PRIVATE
    -Wl,--gc-sections
    -Wl,--exclude-libs,ALL
)