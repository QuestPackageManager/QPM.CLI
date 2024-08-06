# YOU SHOULD NOT MANUALLY EDIT THIS FILE, QPM WILL VOID ALL CHANGES
# always added
target_include_directories(${COMPILE_ID} PRIVATE ${EXTERN_DIR}/includes)
target_include_directories(${COMPILE_ID} SYSTEM PRIVATE ${EXTERN_DIR}/includes/libil2cpp/il2cpp/libil2cpp)

# includes and compile options added by other libraries
# Sadly, there were none with extra include dirs

# libs dir -> stores .so or .a files (or symlinked!)
target_link_directories(${COMPILE_ID} PRIVATE ${EXTERN_DIR}/libs)
RECURSE_FILES(so_list ${EXTERN_DIR}/libs/*.so)
RECURSE_FILES(a_list ${EXTERN_DIR}/libs/*.a)

# every .so or .a that needs to be linked, put here!
# I don't believe you need to specify if a lib is static or not, poggers!
target_link_libraries(${COMPILE_ID} PRIVATE
	${so_list}
	${a_list}
)
