#pragma once
namespace abase { template <class T> class vector {
  T* m_begin; T* m_end; T* m_cap;
 public:
  vector() {} void clear() {} void reserve(unsigned) {} void push_back(const T&) {}
  T& operator[](unsigned) { return *m_begin; }
  const T& operator[](unsigned) const { return *m_begin; } }; }
