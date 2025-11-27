#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

extern "C" {


namespace banette {
namespace ffi {
namespace generator {
namespace openapi {

void generate(const char *openapi_path,
              const char *output_dir,
              const char *file_name,
              const char *module_name,
              const char *extra_headers);

}  // namespace openapi
}  // namespace generator
}  // namespace ffi
}  // namespace banette

}  // extern "C"
