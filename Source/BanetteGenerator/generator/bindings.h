#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

extern "C" {

void generate(const char *openapi_path,
              const char *output_dir,
              const char *file_name,
              const char *module_name);

}  // extern "C"
