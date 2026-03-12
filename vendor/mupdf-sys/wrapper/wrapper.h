/* Windows MSVC does not define max_align_t in C headers (only in C++ <cstddef>).
   Provide the equivalent definition so bindgen can generate the type. */
#if defined(_MSC_VER) && !defined(__cplusplus)
typedef double max_align_t;
#endif

#include "mupdf/fitz.h"
#include "mupdf/ucdn.h"
#include "mupdf/pdf.h"