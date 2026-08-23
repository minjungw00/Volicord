#pragma once

#define ACME_INLINE inline

namespace acme::model {
template <typename T>
struct Box {
    T value;
};

class Named {
public:
    virtual ~Named() = default;
    virtual const char *name() const = 0;
};

class Record : public Named {
public:
    explicit Record(const char *value);
    const char *name() const override;
private:
    const char *value_;
};

const char *normalize(const char *value);
}

namespace acme::other {
class Record {
public:
    int id() const;
};
}
