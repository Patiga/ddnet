#include "base/rust.h"
#include <array>
#include <cassert>
#include <cstddef>
#include <cstdint>
#include <iterator>
#include <new>
#include <stdexcept>
#include <type_traits>
#include <utility>

namespace rust {
inline namespace cxxbridge1 {
// #include "rust/cxx.h"

#ifndef CXXBRIDGE1_PANIC
#define CXXBRIDGE1_PANIC
template <typename Exception>
void panic [[noreturn]] (const char *msg);
#endif // CXXBRIDGE1_PANIC

namespace {
template <typename T>
class impl;
} // namespace

template <typename T>
::std::size_t size_of();
template <typename T>
::std::size_t align_of();

#ifndef CXXBRIDGE1_RUST_SLICE
#define CXXBRIDGE1_RUST_SLICE
namespace detail {
template <bool>
struct copy_assignable_if {};

template <>
struct copy_assignable_if<false> {
  copy_assignable_if() noexcept = default;
  copy_assignable_if(const copy_assignable_if &) noexcept = default;
  copy_assignable_if &operator=(const copy_assignable_if &) &noexcept = delete;
  copy_assignable_if &operator=(copy_assignable_if &&) &noexcept = default;
};
} // namespace detail

template <typename T>
class Slice final
    : private detail::copy_assignable_if<std::is_const<T>::value> {
public:
  using value_type = T;

  Slice() noexcept;
  Slice(T *, std::size_t count) noexcept;

  Slice &operator=(const Slice<T> &) &noexcept = default;
  Slice &operator=(Slice<T> &&) &noexcept = default;

  T *data() const noexcept;
  std::size_t size() const noexcept;
  std::size_t length() const noexcept;
  bool empty() const noexcept;

  T &operator[](std::size_t n) const noexcept;
  T &at(std::size_t n) const;
  T &front() const noexcept;
  T &back() const noexcept;

  Slice(const Slice<T> &) noexcept = default;
  ~Slice() noexcept = default;

  class iterator;
  iterator begin() const noexcept;
  iterator end() const noexcept;

  void swap(Slice &) noexcept;

private:
  class uninit;
  Slice(uninit) noexcept;
  friend impl<Slice>;
  friend void sliceInit(void *, const void *, std::size_t) noexcept;
  friend void *slicePtr(const void *) noexcept;
  friend std::size_t sliceLen(const void *) noexcept;

  std::array<std::uintptr_t, 2> repr;
};

template <typename T>
class Slice<T>::iterator final {
public:
  using iterator_category = std::random_access_iterator_tag;
  using value_type = T;
  using difference_type = std::ptrdiff_t;
  using pointer = typename std::add_pointer<T>::type;
  using reference = typename std::add_lvalue_reference<T>::type;

  reference operator*() const noexcept;
  pointer operator->() const noexcept;
  reference operator[](difference_type) const noexcept;

  iterator &operator++() noexcept;
  iterator operator++(int) noexcept;
  iterator &operator--() noexcept;
  iterator operator--(int) noexcept;

  iterator &operator+=(difference_type) noexcept;
  iterator &operator-=(difference_type) noexcept;
  iterator operator+(difference_type) const noexcept;
  iterator operator-(difference_type) const noexcept;
  difference_type operator-(const iterator &) const noexcept;

  bool operator==(const iterator &) const noexcept;
  bool operator!=(const iterator &) const noexcept;
  bool operator<(const iterator &) const noexcept;
  bool operator<=(const iterator &) const noexcept;
  bool operator>(const iterator &) const noexcept;
  bool operator>=(const iterator &) const noexcept;

private:
  friend class Slice;
  void *pos;
  std::size_t stride;
};

template <typename T>
Slice<T>::Slice() noexcept {
  sliceInit(this, reinterpret_cast<void *>(align_of<T>()), 0);
}

template <typename T>
Slice<T>::Slice(T *s, std::size_t count) noexcept {
  assert(s != nullptr || count == 0);
  sliceInit(this,
            s == nullptr && count == 0
                ? reinterpret_cast<void *>(align_of<T>())
                : const_cast<typename std::remove_const<T>::type *>(s),
            count);
}

template <typename T>
T *Slice<T>::data() const noexcept {
  return reinterpret_cast<T *>(slicePtr(this));
}

template <typename T>
std::size_t Slice<T>::size() const noexcept {
  return sliceLen(this);
}

template <typename T>
std::size_t Slice<T>::length() const noexcept {
  return this->size();
}

template <typename T>
bool Slice<T>::empty() const noexcept {
  return this->size() == 0;
}

template <typename T>
T &Slice<T>::operator[](std::size_t n) const noexcept {
  assert(n < this->size());
  auto ptr = static_cast<char *>(slicePtr(this)) + size_of<T>() * n;
  return *reinterpret_cast<T *>(ptr);
}

template <typename T>
T &Slice<T>::at(std::size_t n) const {
  if (n >= this->size()) {
    panic<std::out_of_range>("rust::Slice index out of range");
  }
  return (*this)[n];
}

template <typename T>
T &Slice<T>::front() const noexcept {
  assert(!this->empty());
  return (*this)[0];
}

template <typename T>
T &Slice<T>::back() const noexcept {
  assert(!this->empty());
  return (*this)[this->size() - 1];
}

template <typename T>
typename Slice<T>::iterator::reference
Slice<T>::iterator::operator*() const noexcept {
  return *static_cast<T *>(this->pos);
}

template <typename T>
typename Slice<T>::iterator::pointer
Slice<T>::iterator::operator->() const noexcept {
  return static_cast<T *>(this->pos);
}

template <typename T>
typename Slice<T>::iterator::reference Slice<T>::iterator::operator[](
    typename Slice<T>::iterator::difference_type n) const noexcept {
  auto ptr = static_cast<char *>(this->pos) + this->stride * n;
  return *reinterpret_cast<T *>(ptr);
}

template <typename T>
typename Slice<T>::iterator &Slice<T>::iterator::operator++() noexcept {
  this->pos = static_cast<char *>(this->pos) + this->stride;
  return *this;
}

template <typename T>
typename Slice<T>::iterator Slice<T>::iterator::operator++(int) noexcept {
  auto ret = iterator(*this);
  this->pos = static_cast<char *>(this->pos) + this->stride;
  return ret;
}

template <typename T>
typename Slice<T>::iterator &Slice<T>::iterator::operator--() noexcept {
  this->pos = static_cast<char *>(this->pos) - this->stride;
  return *this;
}

template <typename T>
typename Slice<T>::iterator Slice<T>::iterator::operator--(int) noexcept {
  auto ret = iterator(*this);
  this->pos = static_cast<char *>(this->pos) - this->stride;
  return ret;
}

template <typename T>
typename Slice<T>::iterator &Slice<T>::iterator::operator+=(
    typename Slice<T>::iterator::difference_type n) noexcept {
  this->pos = static_cast<char *>(this->pos) + this->stride * n;
  return *this;
}

template <typename T>
typename Slice<T>::iterator &Slice<T>::iterator::operator-=(
    typename Slice<T>::iterator::difference_type n) noexcept {
  this->pos = static_cast<char *>(this->pos) - this->stride * n;
  return *this;
}

template <typename T>
typename Slice<T>::iterator Slice<T>::iterator::operator+(
    typename Slice<T>::iterator::difference_type n) const noexcept {
  auto ret = iterator(*this);
  ret.pos = static_cast<char *>(this->pos) + this->stride * n;
  return ret;
}

template <typename T>
typename Slice<T>::iterator Slice<T>::iterator::operator-(
    typename Slice<T>::iterator::difference_type n) const noexcept {
  auto ret = iterator(*this);
  ret.pos = static_cast<char *>(this->pos) - this->stride * n;
  return ret;
}

template <typename T>
typename Slice<T>::iterator::difference_type
Slice<T>::iterator::operator-(const iterator &other) const noexcept {
  auto diff = std::distance(static_cast<char *>(other.pos),
                            static_cast<char *>(this->pos));
  return diff / this->stride;
}

template <typename T>
bool Slice<T>::iterator::operator==(const iterator &other) const noexcept {
  return this->pos == other.pos;
}

template <typename T>
bool Slice<T>::iterator::operator!=(const iterator &other) const noexcept {
  return this->pos != other.pos;
}

template <typename T>
bool Slice<T>::iterator::operator<(const iterator &other) const noexcept {
  return this->pos < other.pos;
}

template <typename T>
bool Slice<T>::iterator::operator<=(const iterator &other) const noexcept {
  return this->pos <= other.pos;
}

template <typename T>
bool Slice<T>::iterator::operator>(const iterator &other) const noexcept {
  return this->pos > other.pos;
}

template <typename T>
bool Slice<T>::iterator::operator>=(const iterator &other) const noexcept {
  return this->pos >= other.pos;
}

template <typename T>
typename Slice<T>::iterator Slice<T>::begin() const noexcept {
  iterator it;
  it.pos = slicePtr(this);
  it.stride = size_of<T>();
  return it;
}

template <typename T>
typename Slice<T>::iterator Slice<T>::end() const noexcept {
  iterator it = this->begin();
  it.pos = static_cast<char *>(it.pos) + it.stride * this->size();
  return it;
}

template <typename T>
void Slice<T>::swap(Slice &rhs) noexcept {
  std::swap(*this, rhs);
}
#endif // CXXBRIDGE1_RUST_SLICE

#ifndef CXXBRIDGE1_RUST_BOX
#define CXXBRIDGE1_RUST_BOX
template <typename T>
class Box final {
public:
  using element_type = T;
  using const_pointer =
      typename std::add_pointer<typename std::add_const<T>::type>::type;
  using pointer = typename std::add_pointer<T>::type;

  Box() = delete;
  Box(Box &&) noexcept;
  ~Box() noexcept;

  explicit Box(const T &);
  explicit Box(T &&);

  Box &operator=(Box &&) &noexcept;

  const T *operator->() const noexcept;
  const T &operator*() const noexcept;
  T *operator->() noexcept;
  T &operator*() noexcept;

  template <typename... Fields>
  static Box in_place(Fields &&...);

  void swap(Box &) noexcept;

  static Box from_raw(T *) noexcept;

  T *into_raw() noexcept;

  /* Deprecated */ using value_type = element_type;

private:
  class uninit;
  class allocation;
  Box(uninit) noexcept;
  void drop() noexcept;

  friend void swap(Box &lhs, Box &rhs) noexcept { lhs.swap(rhs); }

  T *ptr;
};

template <typename T>
class Box<T>::uninit {};

template <typename T>
class Box<T>::allocation {
  static T *alloc() noexcept;
  static void dealloc(T *) noexcept;

public:
  allocation() noexcept : ptr(alloc()) {}
  ~allocation() noexcept {
    if (this->ptr) {
      dealloc(this->ptr);
    }
  }
  T *ptr;
};

template <typename T>
Box<T>::Box(Box &&other) noexcept : ptr(other.ptr) {
  other.ptr = nullptr;
}

template <typename T>
Box<T>::Box(const T &val) {
  allocation alloc;
  ::new (alloc.ptr) T(val);
  this->ptr = alloc.ptr;
  alloc.ptr = nullptr;
}

template <typename T>
Box<T>::Box(T &&val) {
  allocation alloc;
  ::new (alloc.ptr) T(std::move(val));
  this->ptr = alloc.ptr;
  alloc.ptr = nullptr;
}

template <typename T>
Box<T>::~Box() noexcept {
  if (this->ptr) {
    this->drop();
  }
}

template <typename T>
Box<T> &Box<T>::operator=(Box &&other) &noexcept {
  if (this->ptr) {
    this->drop();
  }
  this->ptr = other.ptr;
  other.ptr = nullptr;
  return *this;
}

template <typename T>
const T *Box<T>::operator->() const noexcept {
  return this->ptr;
}

template <typename T>
const T &Box<T>::operator*() const noexcept {
  return *this->ptr;
}

template <typename T>
T *Box<T>::operator->() noexcept {
  return this->ptr;
}

template <typename T>
T &Box<T>::operator*() noexcept {
  return *this->ptr;
}

template <typename T>
template <typename... Fields>
Box<T> Box<T>::in_place(Fields &&...fields) {
  allocation alloc;
  auto ptr = alloc.ptr;
  ::new (ptr) T{std::forward<Fields>(fields)...};
  alloc.ptr = nullptr;
  return from_raw(ptr);
}

template <typename T>
void Box<T>::swap(Box &rhs) noexcept {
  using std::swap;
  swap(this->ptr, rhs.ptr);
}

template <typename T>
Box<T> Box<T>::from_raw(T *raw) noexcept {
  Box box = uninit{};
  box.ptr = raw;
  return box;
}

template <typename T>
T *Box<T>::into_raw() noexcept {
  T *raw = this->ptr;
  this->ptr = nullptr;
  return raw;
}

template <typename T>
Box<T>::Box(uninit) noexcept {}
#endif // CXXBRIDGE1_RUST_BOX

#ifndef CXXBRIDGE1_RUST_OPAQUE
#define CXXBRIDGE1_RUST_OPAQUE
class Opaque {
public:
  Opaque() = delete;
  Opaque(const Opaque &) = delete;
  ~Opaque() = delete;
};
#endif // CXXBRIDGE1_RUST_OPAQUE

#ifndef CXXBRIDGE1_IS_COMPLETE
#define CXXBRIDGE1_IS_COMPLETE
namespace detail {
namespace {
template <typename T, typename = std::size_t>
struct is_complete : std::false_type {};
template <typename T>
struct is_complete<T, decltype(sizeof(T))> : std::true_type {};
} // namespace
} // namespace detail
#endif // CXXBRIDGE1_IS_COMPLETE

#ifndef CXXBRIDGE1_LAYOUT
#define CXXBRIDGE1_LAYOUT
class layout {
  template <typename T>
  friend std::size_t size_of();
  template <typename T>
  friend std::size_t align_of();
  template <typename T>
  static typename std::enable_if<std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_size_of() {
    return T::layout::size();
  }
  template <typename T>
  static typename std::enable_if<!std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_size_of() {
    return sizeof(T);
  }
  template <typename T>
  static
      typename std::enable_if<detail::is_complete<T>::value, std::size_t>::type
      size_of() {
    return do_size_of<T>();
  }
  template <typename T>
  static typename std::enable_if<std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_align_of() {
    return T::layout::align();
  }
  template <typename T>
  static typename std::enable_if<!std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_align_of() {
    return alignof(T);
  }
  template <typename T>
  static
      typename std::enable_if<detail::is_complete<T>::value, std::size_t>::type
      align_of() {
    return do_align_of<T>();
  }
};

template <typename T>
std::size_t size_of() {
  return layout::size_of<T>();
}

template <typename T>
std::size_t align_of() {
  return layout::align_of<T>();
}
#endif // CXXBRIDGE1_LAYOUT

#ifndef CXXBRIDGE1_RELOCATABLE
#define CXXBRIDGE1_RELOCATABLE
namespace detail {
template <typename... Ts>
struct make_void {
  using type = void;
};

template <typename... Ts>
using void_t = typename make_void<Ts...>::type;

template <typename Void, template <typename...> class, typename...>
struct detect : std::false_type {};
template <template <typename...> class T, typename... A>
struct detect<void_t<T<A...>>, T, A...> : std::true_type {};

template <template <typename...> class T, typename... A>
using is_detected = detect<void, T, A...>;

template <typename T>
using detect_IsRelocatable = typename T::IsRelocatable;

template <typename T>
struct get_IsRelocatable
    : std::is_same<typename T::IsRelocatable, std::true_type> {};
} // namespace detail

template <typename T>
struct IsRelocatable
    : std::conditional<
          detail::is_detected<detail::detect_IsRelocatable, T>::value,
          detail::get_IsRelocatable<T>,
          std::integral_constant<
              bool, std::is_trivially_move_constructible<T>::value &&
                        std::is_trivially_destructible<T>::value>>::type {};
#endif // CXXBRIDGE1_RELOCATABLE
} // namespace cxxbridge1
} // namespace rust

struct RustWgpuBackend;

#ifndef CXXBRIDGE1_STRUCT_RustWgpuBackend
#define CXXBRIDGE1_STRUCT_RustWgpuBackend
struct RustWgpuBackend final : public ::rust::Opaque {
  void init_window(::std::uint8_t *window, ::std::uint32_t width, ::std::uint32_t height) noexcept;
  void update_viewport(::std::int32_t x, ::std::int32_t y, ::std::uint32_t w, ::std::uint32_t h, bool by_resize) noexcept;
  void swap() noexcept;
  void render(::std::int32_t blend_mode, ::std::int32_t wrap_mode, ::std::int32_t texture, float screen_tl_x, float screen_tl_y, float screen_br_x, float screen_br_y, bool clipping, ::std::uint32_t clip_x, ::std::uint32_t clip_y, ::std::uint32_t clip_w, ::std::uint32_t clip_h, ::std::uint32_t primitive, ::std::uint32_t primitive_count, ::rust::Slice<const ::std::uint8_t> vertices) noexcept;
  void create_texture(::std::int32_t slot, ::std::uint32_t bytes_per_pixel, ::std::int32_t flags, ::std::uint32_t width, ::std::uint32_t height, ::rust::Slice<const ::std::uint8_t> data) noexcept;
  void update_texture(::std::int32_t slot, ::std::uint32_t x, ::std::uint32_t y, ::std::uint32_t width, ::std::uint32_t height, ::rust::Slice<const ::std::uint8_t> data) noexcept;
  void destroy_texture(::std::int32_t slot) noexcept;
  void clear(double r, double g, double b, double a) noexcept;
  ~RustWgpuBackend() = delete;

private:
  friend ::rust::layout;
  struct layout {
    static ::std::size_t size() noexcept;
    static ::std::size_t align() noexcept;
  };
};
#endif // CXXBRIDGE1_STRUCT_RustWgpuBackend

static_assert(
    ::rust::IsRelocatable<::StrRef>::value,
    "type StrRef should be trivially move constructible and trivially destructible in C++ to be used as a slice element in &[StrRef] in Rust");

extern "C" {
void cxxbridge1$BackendWgpuGreetings(::rust::Slice<const ::StrRef> names) noexcept;
::std::size_t cxxbridge1$RustWgpuBackend$operator$sizeof() noexcept;
::std::size_t cxxbridge1$RustWgpuBackend$operator$alignof() noexcept;

::RustWgpuBackend *cxxbridge1$init_rust_wgpu_backend() noexcept;

void cxxbridge1$RustWgpuBackend$init_window(::RustWgpuBackend &self, ::std::uint8_t *window, ::std::uint32_t width, ::std::uint32_t height) noexcept;

void cxxbridge1$RustWgpuBackend$update_viewport(::RustWgpuBackend &self, ::std::int32_t x, ::std::int32_t y, ::std::uint32_t w, ::std::uint32_t h, bool by_resize) noexcept;

void cxxbridge1$RustWgpuBackend$swap(::RustWgpuBackend &self) noexcept;

void cxxbridge1$RustWgpuBackend$render(::RustWgpuBackend &self, ::std::int32_t blend_mode, ::std::int32_t wrap_mode, ::std::int32_t texture, float screen_tl_x, float screen_tl_y, float screen_br_x, float screen_br_y, bool clipping, ::std::uint32_t clip_x, ::std::uint32_t clip_y, ::std::uint32_t clip_w, ::std::uint32_t clip_h, ::std::uint32_t primitive, ::std::uint32_t primitive_count, ::rust::Slice<const ::std::uint8_t> vertices) noexcept;

void cxxbridge1$RustWgpuBackend$create_texture(::RustWgpuBackend &self, ::std::int32_t slot, ::std::uint32_t bytes_per_pixel, ::std::int32_t flags, ::std::uint32_t width, ::std::uint32_t height, ::rust::Slice<const ::std::uint8_t> data) noexcept;

void cxxbridge1$RustWgpuBackend$update_texture(::RustWgpuBackend &self, ::std::int32_t slot, ::std::uint32_t x, ::std::uint32_t y, ::std::uint32_t width, ::std::uint32_t height, ::rust::Slice<const ::std::uint8_t> data) noexcept;

void cxxbridge1$RustWgpuBackend$destroy_texture(::RustWgpuBackend &self, ::std::int32_t slot) noexcept;

void cxxbridge1$RustWgpuBackend$clear(::RustWgpuBackend &self, double r, double g, double b, double a) noexcept;
} // extern "C"

void BackendWgpuGreetings(::rust::Slice<const ::StrRef> names) noexcept {
  cxxbridge1$BackendWgpuGreetings(names);
}

::std::size_t RustWgpuBackend::layout::size() noexcept {
  return cxxbridge1$RustWgpuBackend$operator$sizeof();
}

::std::size_t RustWgpuBackend::layout::align() noexcept {
  return cxxbridge1$RustWgpuBackend$operator$alignof();
}

::rust::Box<::RustWgpuBackend> init_rust_wgpu_backend() noexcept {
  return ::rust::Box<::RustWgpuBackend>::from_raw(cxxbridge1$init_rust_wgpu_backend());
}

void RustWgpuBackend::init_window(::std::uint8_t *window, ::std::uint32_t width, ::std::uint32_t height) noexcept {
  cxxbridge1$RustWgpuBackend$init_window(*this, window, width, height);
}

void RustWgpuBackend::update_viewport(::std::int32_t x, ::std::int32_t y, ::std::uint32_t w, ::std::uint32_t h, bool by_resize) noexcept {
  cxxbridge1$RustWgpuBackend$update_viewport(*this, x, y, w, h, by_resize);
}

void RustWgpuBackend::swap() noexcept {
  cxxbridge1$RustWgpuBackend$swap(*this);
}

void RustWgpuBackend::render(::std::int32_t blend_mode, ::std::int32_t wrap_mode, ::std::int32_t texture, float screen_tl_x, float screen_tl_y, float screen_br_x, float screen_br_y, bool clipping, ::std::uint32_t clip_x, ::std::uint32_t clip_y, ::std::uint32_t clip_w, ::std::uint32_t clip_h, ::std::uint32_t primitive, ::std::uint32_t primitive_count, ::rust::Slice<const ::std::uint8_t> vertices) noexcept {
  cxxbridge1$RustWgpuBackend$render(*this, blend_mode, wrap_mode, texture, screen_tl_x, screen_tl_y, screen_br_x, screen_br_y, clipping, clip_x, clip_y, clip_w, clip_h, primitive, primitive_count, vertices);
}

void RustWgpuBackend::create_texture(::std::int32_t slot, ::std::uint32_t bytes_per_pixel, ::std::int32_t flags, ::std::uint32_t width, ::std::uint32_t height, ::rust::Slice<const ::std::uint8_t> data) noexcept {
  cxxbridge1$RustWgpuBackend$create_texture(*this, slot, bytes_per_pixel, flags, width, height, data);
}

void RustWgpuBackend::update_texture(::std::int32_t slot, ::std::uint32_t x, ::std::uint32_t y, ::std::uint32_t width, ::std::uint32_t height, ::rust::Slice<const ::std::uint8_t> data) noexcept {
  cxxbridge1$RustWgpuBackend$update_texture(*this, slot, x, y, width, height, data);
}

void RustWgpuBackend::destroy_texture(::std::int32_t slot) noexcept {
  cxxbridge1$RustWgpuBackend$destroy_texture(*this, slot);
}

void RustWgpuBackend::clear(double r, double g, double b, double a) noexcept {
  cxxbridge1$RustWgpuBackend$clear(*this, r, g, b, a);
}

extern "C" {
::RustWgpuBackend *cxxbridge1$box$RustWgpuBackend$alloc() noexcept;
void cxxbridge1$box$RustWgpuBackend$dealloc(::RustWgpuBackend *) noexcept;
void cxxbridge1$box$RustWgpuBackend$drop(::rust::Box<::RustWgpuBackend> *ptr) noexcept;
} // extern "C"

namespace rust {
inline namespace cxxbridge1 {
template <>
::RustWgpuBackend *Box<::RustWgpuBackend>::allocation::alloc() noexcept {
  return cxxbridge1$box$RustWgpuBackend$alloc();
}
template <>
void Box<::RustWgpuBackend>::allocation::dealloc(::RustWgpuBackend *ptr) noexcept {
  cxxbridge1$box$RustWgpuBackend$dealloc(ptr);
}
template <>
void Box<::RustWgpuBackend>::drop() noexcept {
  cxxbridge1$box$RustWgpuBackend$drop(this);
}
} // namespace cxxbridge1
} // namespace rust
